use std::path::PathBuf;

use pitch_detection::detector::mcleod::McLeodDetector;
use pitch_detection::detector::PitchDetector;
use pitch_detection::detector::{autocorrelation::AutocorrelationDetector, yin::YINDetector};
use pitch_detection::float::Float;
use pitch_detection::utils::buffer::new_real_buffer;

#[derive(Debug)]
struct Signal<T> {
    sample_rate: usize,
    data: Vec<T>,
}

#[test]
fn autocorrelation_sin_signal() {
    pure_frequency(String::from("Autocorrelation"), String::from("sin"), 440.0);
}

#[test]
fn mcleod_sin_signal() {
    pure_frequency(String::from("McLeod"), String::from("sin"), 440.0);
}

#[test]
fn yin_sin_signal() {
    pure_frequency(String::from("YIN"), String::from("sin"), 440.0);
}

#[test]
fn autocorrelation_square_signal() {
    pure_frequency(
        String::from("Autocorrelation"),
        String::from("square"),
        440.0,
    );
}

#[test]
fn mcleod_square_signal() {
    pure_frequency(String::from("McLeod"), String::from("square"), 440.0);
}

#[test]
fn yin_square_signal() {
    pure_frequency(String::from("YIN"), String::from("square"), 440.0);
}

#[test]
fn autocorrelation_triangle_signal() {
    pure_frequency(
        String::from("Autocorrelation"),
        String::from("triangle"),
        440.0,
    );
}

#[test]
fn mcleod_triangle_signal() {
    pure_frequency(String::from("McLeod"), String::from("triangle"), 440.0);
}

#[test]
fn yin_triangle_signal() {
    pure_frequency(String::from("YIN"), String::from("triangle"), 440.0);
}

#[test]
fn autocorrelation_violin_d4() {
    let signal: Signal<f64> = wav_file_to_signal(samples_path("violin-D4.wav"), 0, 10 * 1024);

    raw_frequency("Autocorrelation".into(), signal, 293.);
}

#[test]
fn mcleod_violin_d4() {
    let signal: Signal<f64> = wav_file_to_signal(samples_path("violin-D4.wav"), 0, 10 * 1024);

    raw_frequency("McLeod".into(), signal, 293.);
}

#[test]
fn autocorrelation_violin_f4() {
    let signal: Signal<f64> = wav_file_to_signal(samples_path("violin-F4.wav"), 0, 10 * 1024);

    raw_frequency("Autocorrelation".into(), signal, 349.);
}

#[test]
fn mcleod_violin_f4() {
    let signal: Signal<f64> = wav_file_to_signal(samples_path("violin-F4.wav"), 0, 10 * 1024);

    raw_frequency("McLeod".into(), signal, 349.);
}

#[test]
fn autocorrelation_violin_g4() {
    let signal: Signal<f64> = wav_file_to_signal(samples_path("violin-G4.wav"), 0, 10 * 1024);

    raw_frequency("Autocorrelation".into(), signal, 392.);
}

#[test]
fn mcleod_violin_g4() {
    let signal: Signal<f64> = wav_file_to_signal(samples_path("violin-G4.wav"), 0, 10 * 1024);

    raw_frequency("McLeod".into(), signal, 392.);
}

#[test]
fn mcleod_tenor_trombone_c3() {
    let signal: Signal<f64> =
        wav_file_to_signal(samples_path("tenor-trombone-C3.wav"), 0, 10 * 1024);

    raw_frequency("McLeod".into(), signal, 130.);
}

#[test]
fn mcleod_tenor_trombone_db3() {
    let signal: Signal<f64> =
        wav_file_to_signal(samples_path("tenor-trombone-Db3.wav"), 0, 10 * 1024);

    raw_frequency("McLeod".into(), signal, 138.);
}

#[test]
fn mcleod_tenor_trombone_ab3() {
    let signal: Signal<f64> =
        wav_file_to_signal(samples_path("tenor-trombone-Ab3.wav"), 0, 10 * 1024);

    raw_frequency("McLeod".into(), signal, 207.);
}

#[test]
fn mcleod_tenor_trombone_b3() {
    let signal: Signal<f64> =
        wav_file_to_signal(samples_path("tenor-trombone-B3.wav"), 0, 10 * 1024);

    raw_frequency("McLeod".into(), signal, 246.);
}

fn get_chunk<T: Float>(signal: &[T], start: usize, window: usize, output: &mut [T]) {
    let start = match signal.len() > start {
        true => start,
        false => signal.len(),
    };

    let stop = match signal.len() >= start + window {
        true => start + window,
        false => signal.len(),
    };

    output[..(stop - start)].copy_from_slice(&signal[start..((stop - start) + start)]);

    for o in output.iter_mut().skip(stop - start) {
        *o = T::zero();
    }
}

fn wav_file_to_signal<T: Float>(
    file_name: String,
    seek_start: usize,
    num_samples: usize,
) -> Signal<T> {
    println!("Opening \"{}\"", file_name);
    let mut reader = hound::WavReader::open(file_name).unwrap();
    let sample_rate = reader.spec().sample_rate as usize;
    let data: Vec<T> = reader
        .samples::<i32>()
        .skip(seek_start)
        .map(|s| T::from_i32(s.unwrap()).unwrap())
        .take(num_samples)
        .collect();

    Signal { sample_rate, data }
}

/// Get the full path of `wav` file specified by `file_name`.
fn samples_path(file_name: &str) -> String {
    // `d` is an absolute path to the source directory of the project
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // all audio samples are in this subfolder
    d.push("tests/samples");
    d.push(file_name);

    d.to_str().unwrap().into()
}

fn sin_wave<T: Float>(freq: f64, size: usize, sample_rate: usize) -> Vec<T> {
    let mut signal = new_real_buffer(size);
    let two_pi = 2.0 * std::f64::consts::PI;
    let dx = two_pi * freq / sample_rate as f64;
    for (i, s) in signal.iter_mut().enumerate().take(size) {
        let x = i as f64 * dx;
        let y = x.sin();
        *s = T::from(y).unwrap();
    }
    signal
}

fn square_wave<T: Float>(freq: f64, size: usize, sample_rate: usize) -> Vec<T> {
    let mut signal = new_real_buffer(size);
    let period = sample_rate as f64 / freq;

    for (i, s) in signal.iter_mut().enumerate().take(size) {
        let x = i as f64 / period;
        let frac = x - x.floor();
        let y = match frac >= 0.5 {
            true => -1.0,
            false => 1.0,
        };
        *s = T::from(y).unwrap();
    }
    signal
}

fn triangle_wave<T: Float>(freq: f64, size: usize, sample_rate: usize) -> Vec<T> {
    let mut signal = new_real_buffer(size);
    let period = sample_rate as f64 / freq;

    for (i, s) in signal.iter_mut().enumerate().take(size) {
        let x = i as f64 / period;
        let frac = x - x.floor();
        let y = match frac {
            f if (0. ..0.25).contains(&f) => 4. * f,
            f if (0.25..0.75).contains(&f) => 1. - 4. * (f - 0.25),
            f if (0.75..1.9).contains(&f) => -1. + 4. * (f - 0.75),
            _ => panic!("Should be between 0 and 1"),
        };
        *s = T::from(y).unwrap();
    }
    signal
}

fn saw_wave<T: Float>(freq: f64, size: usize, sample_rate: usize) -> Vec<T> {
    let mut signal = new_real_buffer(size);
    let period = sample_rate as f64 / freq;

    for (i, s) in signal.iter_mut().enumerate().take(size) {
        let x = i as f64 / period;
        let frac = x - x.floor();
        let y = match frac {
            f if (0.0..0.25).contains(&f) => 4. * f,
            f if (0.25..0.75).contains(&f) => -1. + 4. * (f - 0.25),
            f if (0.75..1.).contains(&f) => -1. + 4. * (f - 0.75),
            _ => panic!("Should be between 0 and 1"),
        };
        *s = T::from(y).unwrap();
    }
    signal
}

fn detector_factory(name: String, window: usize, padding: usize) -> Box<dyn PitchDetector<f64>> {
    match name.as_ref() {
        "McLeod" => Box::new(McLeodDetector::<f64>::new(window, padding)),
        "Autocorrelation" => Box::new(AutocorrelationDetector::<f64>::new(window, padding)),
        "YIN" => Box::new(YINDetector::<f64>::new(window, padding)),
        _ => {
            panic!("Unknown detector {}", name);
        }
    }
}

fn signal_factory<T: Float>(name: String, freq: f64, size: usize, sample_rate: usize) -> Vec<T> {
    match name.as_ref() {
        "sin" => sin_wave(freq, size, sample_rate),
        "square" => square_wave(freq, size, sample_rate),
        "triangle" => triangle_wave(freq, size, sample_rate),
        "saw" => saw_wave(freq, size, sample_rate),
        _ => {
            panic!("Unknown wave function {}", name);
        }
    }
}

fn pure_frequency(detector_name: String, wave_name: String, freq_in: f64) {
    const SAMPLE_RATE: usize = 48000;
    const DURATION: f64 = 4.0;
    const SAMPLE_SIZE: usize = (SAMPLE_RATE as f64 * DURATION) as usize;
    const WINDOW: usize = 1024;
    const PADDING: usize = WINDOW / 2;
    const DELTA_T: usize = WINDOW / 4;
    const N_WINDOWS: usize = (SAMPLE_SIZE - WINDOW) / DELTA_T;
    const POWER_THRESHOLD: f64 = 300.0;
    const CLARITY_THRESHOLD: f64 = 0.6;

    let signal = signal_factory::<f64>(wave_name, freq_in, SAMPLE_SIZE, SAMPLE_RATE);

    let mut chunk = new_real_buffer(WINDOW);

    let mut detector = detector_factory(detector_name, WINDOW, PADDING);

    for i in 0..N_WINDOWS {
        let t: usize = i * DELTA_T;
        get_chunk(&signal, t, WINDOW, &mut chunk);

        let pitch = detector.get_pitch(&chunk, SAMPLE_RATE, POWER_THRESHOLD, CLARITY_THRESHOLD);

        match pitch {
            Some(pitch) => {
                let frequency = pitch.frequency;
                let clarity = pitch.clarity;
                let idx = SAMPLE_RATE as f64 / frequency;
                let epsilon = (SAMPLE_RATE as f64 / (idx - 1.0)) - frequency;
                println!(
                    "Chosen Peak idx: {}; clarity: {}; freq: {} +/- {}",
                    idx, clarity, frequency, epsilon
                );
                assert!((frequency - freq_in).abs() < 2. * epsilon);
            }
            None => {
                println!("No peaks accepted.");
                panic!();
            }
        }
    }
}

/// Test if the signal in `signal` is reasonably close to `freq_in`.
fn raw_frequency(detector_name: String, signal: Signal<f64>, freq_in: f64) {
    const ERROR_TOLERANCE: f64 = 2.;
    let sample_rate = signal.sample_rate;
    let duration: f64 = signal.data.len() as f64 / sample_rate as f64;
    let sample_size: usize = (signal.sample_rate as f64 * duration) as usize;
    const WINDOW: usize = 1024;
    const PADDING: usize = WINDOW / 2;
    const DELTA_T: usize = WINDOW / 4;
    let n_windows: usize = (sample_size - WINDOW) / DELTA_T;
    const POWER_THRESHOLD: f64 = 300.0;
    const CLARITY_THRESHOLD: f64 = 0.6;

    let mut chunk = new_real_buffer(WINDOW);

    let mut detector = detector_factory(detector_name, WINDOW, PADDING);

    for i in 0..n_windows {
        let t: usize = i * DELTA_T;
        get_chunk(&signal.data, t, WINDOW, &mut chunk);

        let pitch = detector.get_pitch(&chunk, sample_rate, POWER_THRESHOLD, CLARITY_THRESHOLD);

        match pitch {
            Some(pitch) => {
                let frequency = pitch.frequency;
                let clarity = pitch.clarity;
                let idx = sample_rate as f64 / frequency;
                let epsilon = (sample_rate as f64 / (idx - 1.0)) - frequency;
                println!(
                    "Chosen Peak idx: {}; clarity: {}; freq: {} +/- {}",
                    idx, clarity, frequency, epsilon
                );
                assert!((frequency - freq_in).abs() < ERROR_TOLERANCE * epsilon);
            }
            None => {
                println!("No peaks accepted.");
                panic![];
            }
        }
    }
}
