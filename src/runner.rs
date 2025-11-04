//! The function `run(mpsc::Sender<NoteDetectionResult>, &str)` opens
//! a Jackd AudioIn pipe, samples audio from it, and then sends
//! analysis of the audio in the form of `NoteDetectionResult` through
//! the first argument.  See the [example](../examples/detect_note.rs)
use crate::detector::autocorrelation::AutocorrelationDetector;
use crate::detector::mcleod::McLeodDetector;
use crate::detector::yin::YINDetector;
use crate::detector::PitchDetector;
use crate::note_detection_result::NoteDetectionResult;
use crate::Pitch;
use jack::{
    AsyncClient, AudioIn, Client, ClientOptions, Control, Port, ProcessHandler, ProcessScope,
};
use std::error::Error;
use std::fmt::{self, Formatter};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const SLEEP_MS: u64 = 300;

/// Handle Jackd notifications.
pub struct JackNotifications;
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
pub struct JackProcessHandlerPD {
    capture_port: Port<AudioIn>,
    tx: Sender<f32>,
}

impl ProcessHandler for JackProcessHandlerPD {
    /// Call back for Jack to put audio data n the ring bufer
    fn process(&mut self, c: &Client, ps: &ProcessScope) -> jack::Control {
        let buffer = self.capture_port.as_slice(ps);
        for b in buffer {
            if let Err(err) = self.tx.send(*b) {
                eprintln!(
		    "Error pitch_detector/runner: Cannot send data from Jack client: {}.  Error: {err}",
		    c.name()
		);
                return jack::Control::Quit;
            }
        }
        jack::Control::Continue
    }
}

impl JackProcessHandlerPD {}

/// The meta data the detector needs
#[derive(Debug, Clone)]
pub enum Detector {
    McLeod,
    AutoCorrelation,
    Yin,
}
#[derive(Debug, Clone)]
pub struct DetectorCfg {
    pub sample_rate: u32,
    pub size: usize, // Number of samples to use to detect pitch
    pub padding: usize,
    pub power_threshold: f32,
    pub clarity_threshold: f32,
    pub detector: Detector,
}

impl fmt::Display for DetectorCfg {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{:>6} {:>6} {:>6} {:>6.3} {:>6.3} {:?}",
            self.sample_rate,
            self.size,
            self.padding,
            self.power_threshold,
            self.clarity_threshold,
            self.detector,
        )
    }
}

/// Get the pitch
fn my_get_pitch(
    signal: &[f32],
    detector: &mut dyn PitchDetector<f32>,
    sample_rate: u32,
    power_threshold: f32,
    clarity_threshold: f32,
) -> Option<Pitch<f32>> {
    detector.get_pitch(
        signal,
        sample_rate as usize,
        power_threshold,
        clarity_threshold,
    )
}

/// Start the Jack client thread that will provide audio to the pitch
/// detector.  The audio data is received on `src_port` and sent via
/// `tx`
pub fn start_jack(
    tx: Sender<f32>,
    src_port: &str,
) -> Result<AsyncClient<JackNotifications, JackProcessHandlerPD>, Box<dyn Error>> {
    let client_name = "qzn3t_pitch_detector";
    let port_name = "input";

    let (client, _) = jack::Client::new(client_name, ClientOptions::NO_START_SERVER).unwrap();
    let capture_port = client.register_port(port_name, AudioIn::default()).unwrap();
    // Connect the audio ports
    let dst_port = capture_port
        .name()
        .expect("Error pitch_detection/runner start_jack: Cannot get dst_port");
    let handler = JackProcessHandlerPD { capture_port, tx };
    let ac = client.activate_async(JackNotifications, handler).unwrap();

    if let Err(err) = ac
        .as_client()
        .connect_ports_by_name(src_port, dst_port.as_str())
    {
        panic!("Error tuner: Connecting {src_port} -> {dst_port}  failed. {err}",);
    }

    Ok(ac)
}

/// Get data from from Jack on `rx` jack port and analyze its pitch.
/// Send pitch data, continuously, on `tx`.  The configuration for the
/// pitch detector is in `detector_cfg` and `kill_switch` is used to
/// stop the process
pub fn pitch_detection_run(
    tx: Sender<NoteDetectionResult>,
    rx: Receiver<f32>,
    detector_cfg: &DetectorCfg,
    kill_switch: Arc<Mutex<bool>>,
) -> JoinHandle<()> {
    let detector = detector_cfg.detector.clone();
    let buf_sz = detector_cfg.size;
    let padding = detector_cfg.padding;
    let sample_rate = detector_cfg.sample_rate;
    let power_threshold = detector_cfg.power_threshold;
    let clarity_threashold = detector_cfg.clarity_threshold;
    let jh = thread::spawn(move || {
        let sleep_ms = SLEEP_MS;

        // The pitch detector to use
        let mut detector: Box<dyn PitchDetector<f32>> = match detector {
            Detector::McLeod => Box::new(McLeodDetector::new(buf_sz, padding)),
            Detector::AutoCorrelation => Box::new(AutocorrelationDetector::new(buf_sz, padding)),
            Detector::Yin => Box::new(YINDetector::new(buf_sz, padding)),
        };

        // Buffer to hold samples.
        let mut samples = Vec::with_capacity(buf_sz);
        loop {
            let top_of_loop = Instant::now();
            {
                // Check for exit condition.
                if *kill_switch.lock().unwrap() {
                    break;
                }
            }

            // Fill the buffer.  Need to ensure that the latest data is
            // being used so read data from `rx` until there have been
            // `buf_sz` bytes read and there are no more to read from
            // `rx`
            samples.clear();
            loop {
                let all_values: Vec<f32> = rx.try_iter().collect();
                samples.extend_from_slice(&all_values);
                if samples.len() >= buf_sz {
                    samples = samples[samples.len() - buf_sz..].to_vec();
                    break;
                }
            }

            // Do the deed with the samples from Jack and send the
            // result back to the caller
            let pitch = my_get_pitch(
                &samples,
                &mut *detector,
                sample_rate,
                power_threshold,
                clarity_threashold,
            );

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
                if let Err(err) = tx.send(ndr) {
                    eprintln!("Error pitch_detection/runner/pitch_detection_run: {err}");
                    break;
                }
            }

            // Keep the speed of detection
            let sleep = sleep_ms as i64 - top_of_loop.elapsed().as_millis() as i64;
            if sleep > 0 {
                thread::sleep(Duration::from_millis(sleep as u64));
            } else if sleep < 0 {
                eprintln!(
                    "DBG pitch_detection/runner: Detection loop over ran: {}ms of {sleep_ms}ms",
                    -sleep
                );
            }
        }
    });
    jh
}
