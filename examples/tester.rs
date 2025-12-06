//! Test configurations over a set of samples of musical instruments
//! against the actual note.  The data to input is raw audio, taken
//! directly from a Jack pipe, the Note/Octave ("E/2" "F#/4" etcetera)
//! and the cents defining accuracy -50.0..50.0
//! Command line, two arguments:
//!
//!   1. Sample rate
//!   2. Path to input file
//!
//! Input is CSV
//!
//! Four fields:
//!   1: An index so results can be matched to input
//!   2: The actual note ("E/2" "F#/4" etcetera)
//!   3: The cents (-50.0..50).
//!   4: Path to the file of raw data
use clap::Parser;
use jack::{AudioOut, Port};
#[allow(unused_imports)]
use jack::{Client, ClientOptions, Control, ProcessHandler, ProcessScope};
use pitch_detection::note_detection_result::{NoteDetectionResult, NoteName};
use pitch_detection::runner;
#[allow(unused_imports)]
use pitch_detection::runner::{pitch_detection_run, Detector, DetectorCfg};
use pitch_detection::rx_proxy::RxProxy;
use std::fmt::Debug;
use std::sync::mpsc::channel;
use std::sync::{mpsc, Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use std::{f32, fs};
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    path: String,
    #[arg(short, long)]
    sample_rate: u32,
}

/// The structure for sending samples out a Jack audio port
struct OutProcess {
    samples: Arc<Mutex<Vec<f32>>>,
    output: Port<AudioOut>,
    position: usize,
    state: Arc<Mutex<OutProcessState>>,
}

/// Status of the inner jack loop that is taking data from the audio
/// connection, buffering it, detecting the pitch then sending the
/// pitch data out on a channel.
#[derive(PartialEq)]
enum OutProcessState {
    Running,
    Paused,
    Quit,
}

impl ProcessHandler for OutProcess {
    fn process(&mut self, c: &Client, ps: &ProcessScope) -> Control {
        {
            if *self.state.lock().unwrap() == OutProcessState::Paused {
                return Control::Continue;
            }
        }
        let output = self.output.as_mut_slice(ps);
        let frames = output.len();
        let mut local_samples: Vec<f32> = {
            let samples_guard: MutexGuard<'_, Vec<f32>> = self.samples.lock().unwrap();
            samples_guard.clone()
        };
        let mut len: usize; // Number of samples
        let sample_rate = c.sample_rate();
        // Every 100ms refresh the sample buffer.
        // let sz = sample_rate / 10;
        let sz = sample_rate;

        // The samples can change between invocations.  So
        // self.position must be checked for validity
        if self.position >= local_samples.len() {
            self.position = 0;
        }
        for (i, _) in (0..frames).enumerate() {
            if local_samples.is_empty() {
                output[i] = 0_f32;
                continue;
            }
            if self.position >= local_samples.len() {
                panic!(
                    "PANIC: position: {}, length: {} Frame: {i}",
                    self.position,
                    local_samples.len()
                );
            }
            if self.position % sz == 0 {
                local_samples = {
                    let samples_guard: MutexGuard<'_, Vec<f32>> = self.samples.lock().unwrap();
                    samples_guard.clone()
                };
                len = local_samples.len();
                if self.position >= len {
                    self.position = 0;
                }
            }
            if self.position >= local_samples.len() {
                panic!(
                    "PANIC: position: {}, length: {}",
                    self.position,
                    local_samples.len()
                );
            }
            let sample = local_samples[self.position];

            output[i] = sample;
            self.position += 1;
            if self.position == local_samples.len() {
                // Looping
                self.position = 0;
            }
        }
        if *self.state.lock().unwrap() == OutProcessState::Quit {
            Control::Quit
        } else {
            Control::Continue
        }
    }
}

#[allow(dead_code)]
struct TestResult {
    note: NoteName,
    octave: u32,
    cents: f32,
}

#[allow(dead_code)]
fn test_detection() -> Vec<TestResult> {
    // The pitch detection channels.
    vec![]
}
const TEST_DURATION: Duration = Duration::from_secs(4);
fn main() {
    let args = Args::parse();
    #[allow(unused_variables)]
    let (tx, rx) = mpsc::channel::<NoteDetectionResult>();
    // The lines specifying the test cases
    let lines = match fs::read_to_string(&args.path) {
        Ok(contents) => contents
            .lines()
            .map(|s| s.to_string())
            .collect::<Vec<String>>(),
        Err(e) => panic!("Error reading file '{}': {}", args.path, e),
    };

    // The sample data.  None yet
    let samples = Arc::new(Mutex::<Vec<f32>>::new(vec![]));

    // The state of the inner loop processing audio data into pitch data
    let state: Arc<Mutex<OutProcessState>> = Arc::new(Mutex::new(OutProcessState::Running));
    // Over each test case for each value of the parameters
    // for line in lines {

    // Parameters to be optimised
    let models = [Detector::McLeod, Detector::Yin, Detector::AutoCorrelation];
    let power_thresholds = [0.1, 1.0, 10.0];
    let clarity_thresholds = [0.3, 0.5, 0.9];
    let sizes = [1024, 4096, 16384]; // Size of the sample for detection
    let paddings = [512, 256, 1024]; // TODO: Document
    #[derive(Debug)]
    struct TestCase {
        samples: Vec<f32>,
        octave: u32,
        cents: f32,
        index: u32,
        note: NoteName,
        path: String,
    }
    let mut test_cache: Vec<TestCase> = Vec::new();

    for line in lines.iter() {
        let fields = line.split(',').map(|c| c.trim()).collect::<Vec<&str>>();
        assert!(fields.len() == 5);
        let index: u32 = fields[0].parse().expect("Index");
        let note: NoteName = fields[1].into();
        let octave: u32 = fields[2].parse().expect("Octave");
        let cents: f32 = fields[3].parse().expect("Cents");
        let path = fields[4];
        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(err) => panic!(
		"Error qzn3t_pitch_detection/tester: Failed to load samples. Path: {path}  Error: {err}"
	    ),
        };
        let these_samples: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|chunk| {
                let bytes_array: [u8; 4] = chunk.try_into().unwrap();
                f32::from_ne_bytes(bytes_array)
            })
            .collect();
        test_cache.push(TestCase {
            samples: these_samples,
            octave,
            cents,
            note,
            index,
            path: path.to_string(),
        });
    }

    let (client, _status) =
        jack::Client::new("qzn3t_sample_source", ClientOptions::NO_START_SERVER).unwrap();
    // Output audio port
    let output = client
        .register_port("outout", AudioOut::default())
        .expect("Error qzn3t_pitch_detection: Output port");
    let output_name = output.name().unwrap();

    let out_process = OutProcess {
        samples: samples.clone(),
        output,
        position: 0,
        state: state.clone(),
    };
    // Set up a Jack client to generate the audio
    let audio_src_client = client
        .activate_async(runner::JackNotifications, out_process)
        .unwrap();

    // The channel that audio data will be moved from Jack to pitch detection
    let (tx_f32, rx_f32) = mpsc::channel::<f32>();

    // Set up the pitch detection Jack client
    let audio_dst_client = match runner::start_jack(tx_f32, &output_name) {
        Ok(ac) => ac,
        Err(err) => panic!(
            "Error pitch_detectiopn tester: Cannot create Jack clent to receive audio: {err}"
        ),
    };

    let mut rx_proxy = RxProxy::new(rx_f32);
    for model in models.iter() {
        for size in sizes {
            for padding in paddings {
                for power_threshold in power_thresholds {
                    for clarity_threshold in clarity_thresholds {
                        // Configure a detector
                        let detector_cfg = DetectorCfg {
                            sample_rate: args.sample_rate,
                            size,
                            padding,
                            power_threshold,
                            clarity_threshold,
                            detector: model.clone(),
                        };
                        let msg = format!(
                            "Detector Configuration:  {:>4} {:>4} {:>6.3} {:>6.3} {:?}",
                            detector_cfg.size,
                            detector_cfg.padding,
                            detector_cfg.power_threshold,
                            detector_cfg.clarity_threshold,
                            detector_cfg.detector,
                        );
                        println!("{msg}");

                        // The channel that pitch data will be received on
                        let (tx_ndr, rx_ndr) = mpsc::channel::<NoteDetectionResult>();
                        let pitch_detector_kill_switch = Arc::new(Mutex::new(false));

                        // Set up channles to proxy audio data
                        let (tx_p, rx_p) = channel::<f32>();
                        rx_proxy.set_sender(tx_p);
                        let jh = pitch_detection_run(
                            tx_ndr,
                            rx_p,
                            &detector_cfg,
                            Some(pitch_detector_kill_switch.clone()),
                        );

                        for test_case in test_cache.iter() {
                            // Case to test
                            {
                                *samples.lock().unwrap() = test_case.samples.clone();
                                // let s = samples.lock().unwrap();
                                // let sum = s.iter().fold(0.0_f32, |a, b| a + *b);
                                // let len = s.len() as f32;
                                // let mean = sum / len;
                                // let max = s.iter().fold(0.0_f32, |a, &b| if a > b { a } else { b });
                                // let min = s.iter().fold(0.0_f32, |a, &b| if a < b { a } else { b });
                                // eprintln!(
                                //     "DBG Test case samples: mean: {mean} max: {max} min: {min} length: {len}",
                                // );
                            }

                            let index = test_case.index;
                            let true_cents = test_case.cents;
                            let true_note = test_case.note.clone();
                            let true_octave = test_case.octave;
                            println!(
				"Test case {index:>3} {true_note:>2}/{true_octave} {true_cents:>-6.3} {}",
				test_case.path
			    );
                            let now = Instant::now();
                            loop {
                                if now.elapsed() > TEST_DURATION {
                                    break;
                                }
                                let ndr = match rx_ndr.recv_timeout(TEST_DURATION) {
                                    Ok(ndr) => ndr,
                                    Err(mpsc::RecvTimeoutError::Timeout) => {
                                        continue;
                                    }
                                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                                        break;
                                    }
                                };
                                let detect_cents = ndr.cents;
                                let detect_note = ndr.note_name;
                                let detect_octave = ndr.octave;
                                println!("Result: {index:>3} {true_note:>2}/{detect_note:>2} {true_octave}/{detect_octave} {true_cents:6.3}/{detect_cents:6.3}");
                            }
                        }

                        {
                            // Stop this pitch detector
                            *pitch_detector_kill_switch.lock().unwrap() = true;
                        }
                        _ = jh.join();
                    }
                }
            }
        }
    }
    _ = audio_src_client.deactivate();
    _ = audio_dst_client.deactivate();
    {
        *state.lock().unwrap() = OutProcessState::Quit;
    };
}
