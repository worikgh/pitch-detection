//! The function `run(mpsc::Sender<NoteDetectionResult>, &str)` opens
//! a Jackd AudioIn pipe, samples audio from it, and then sends
//! analysis of the audio in the form of `NoteDetectionResult` through
//! the first argument.  See the [example](../examples/detect_note.rs)
use crate::detector::mcleod::McLeodDetector;
use crate::detector::PitchDetector;
use crate::note_detection_result::NoteDetectionResult;
use crate::Pitch;
use jack::{AudioIn, Client, Control, Port, ProcessHandler, ProcessScope};
use ringbuf::traits::{Consumer, Observer, Producer};
use ringbuf::HeapRb;
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const RING_BUFFER_CAPACITY: usize = 2_048_000;
const SAMPLE_SIZE: usize = 10240;
const SLEEP_MS: u64 = 300;

/// Handle Jackd notifications.
struct JackNotifications;
impl jack::NotificationHandler for JackNotifications {
    // Accept most defaults

    /// It is worth noting xruns.
    fn xrun(&mut self, _: &Client) -> Control {
        eprintln!("DBG detect_pitch:   xrun");
        Control::Continue
    }
}

/// This is passed to Jackd.  The shared ring buffer is filled with
/// audio data for periodically passing to the pitch detector
struct JackProcessHandlerRB {
    capture_port: Port<AudioIn>,
    ring_buffer: Arc<Mutex<HeapRb<f32>>>,
}

impl ProcessHandler for JackProcessHandlerRB {
    /// Call back for Jack to put audio data n the ring bufer
    fn process(&mut self, _: &Client, ps: &ProcessScope) -> jack::Control {
        let buffer = self.capture_port.as_slice(ps);

        // Push all available samples to the ring buffer
        let rb_guard = self.ring_buffer.lock().unwrap();
        let mut rb = rb_guard;
        let mut source_iter = buffer.iter().copied();
        let pushed_count = rb.push_iter(&mut source_iter);

        if pushed_count < buffer.len() {
            eprintln!(
                "Ring buffer full, dropped {} samples",
                buffer.len() - pushed_count
            );
        }
        jack::Control::Continue
    }
}

/// The meta data the detector needs
pub struct DetectorCfg<T> {
    sample_rate: usize,
    size: usize,
    padding: usize,
    power_threshold: T,
    clarity_threshold: T,
}

/// Get the pitch
fn my_get_pitch<T: crate::float::Float, U: PitchDetector<T>>(
    signal: &[T],
    detector: &mut U,
    cfg: &DetectorCfg<T>,
) -> Option<Pitch<T>> {
    detector.get_pitch(
        signal,
        cfg.sample_rate,
        cfg.power_threshold,
        cfg.clarity_threshold,
    )
}

/// Get data from a jack port and analyze its pitch.  Send pitch data,
/// continuously, to `sender`
pub fn run(sender: mpsc::Sender<NoteDetectionResult>, input: &str) -> JoinHandle<()> {
    // Get Jack client
    let (client, _status) =
        jack::Client::new("qzn3t_detect_pitch", jack::ClientOptions::NO_START_SERVER).unwrap();
    let sample_rate = client.sample_rate();
    let sample_size = SAMPLE_SIZE; // The number of samples to send to the detector
    let detector_cfg = DetectorCfg {
        sample_rate,
        size: sample_size,
        padding: 1024 / 2,
        power_threshold: 5.0,
        clarity_threshold: 0.7,
    };

    let connect_port = input.to_string();
    thread::spawn(move || {
        // Create ring buffer with specified capacity
        let ring_buffer = Arc::new(Mutex::new(HeapRb::<f32>::new(RING_BUFFER_CAPACITY)));
        let ring_buffer_clone = Arc::clone(&ring_buffer);

        // Register capture port
        let capture_port = client.register_port("input", AudioIn::default()).unwrap();
        let capture_port_name = capture_port.name().unwrap();

        // Activate the client with our custom handler
        let handler = JackProcessHandlerRB {
            capture_port,
            ring_buffer: ring_buffer_clone,
        };
        let active_client = client.activate_async(JackNotifications, handler).unwrap();

        // A port to connect to the tuner was specified so make the connection
        let client = active_client.as_client();
        let capture_port = client.port_by_name(&capture_port_name).unwrap();
        let c_port = match client.port_by_name(connect_port.as_str()) {
            Some(p) => p,
            None => panic!(
                "Error tuner: start_jack_thread: Conection port: {connect_port} is unavailable.  "
            ),
        };
        if let Err(err) = client.connect_ports(&c_port, &capture_port) {
            panic!(
                "Error tuner: Connecting {:?} -> {:?}  failed. {err}",
                c_port, capture_port,
            );
        }

        let sleep_ms = SLEEP_MS;
        let mut detector = McLeodDetector::new(detector_cfg.size, detector_cfg.padding);

        let mut samples = Vec::with_capacity(sample_size);
        loop {
            let sample_interval = Duration::from_millis(sleep_ms);
            thread::sleep(sample_interval);

            // Get available samples from the ring buffer
            samples.resize(sample_size, 0.0);
            let mut rb_guard = ring_buffer.lock().unwrap();
            let available = (*rb_guard).occupied_len();
            if available < sample_size {
                continue;
            }
            let sz_popped = (*rb_guard).pop_slice(&mut samples);
            if sz_popped != sample_size {
                eprintln!("Error pitch_detector: There were {sz_popped} bytes popped from ring buffer.  There should have been {sample_size}.  Available is: {available}");
                continue;
            }

            // Empty the buffer now it has been used
            (*rb_guard).clear();
            drop(rb_guard);

            // Do the deed with the samples from Jack and send the
            // result back to the caller
            let pitch = my_get_pitch(&samples, &mut detector, &detector_cfg);
            if let Some(pitch) = pitch {
                let freq = pitch.frequency;
                let clarity = pitch.clarity;
                let ndr: NoteDetectionResult =
                    match NoteDetectionResult::from_freq_clarity(freq, clarity) {
                        Ok(ndr) => ndr,
                        Err(err) => {
                            eprintln!("Error pitch-detection: {err}");
                            continue;
                        }
                    };
                sender.send(ndr).unwrap();
            }
        }
    })
}
