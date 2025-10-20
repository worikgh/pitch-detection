use pitch_detection::detector::internals::Pitch;
use pitch_detection::detector::mcleod::McLeodDetector;
use pitch_detection::detector::PitchDetector;
struct DetectorCfg<T> {
    sample_rate: usize,
    size: usize,
    padding: usize,
    power_threshold: T,
    clarity_threshold: T,
}

fn main() {
    // const SAMPLE_RATE: usize = 48000;
    // const SIZE: usize = 1024;
    // const PADDING: usize = SIZE / 2;
    // const POWER_THRESHOLD: f32 = 5.0;
    // const CLARITY_THRESHOLD: f32 = 0.7;
    let detector_cfg = DetectorCfg {
        sample_rate: 48_000,
        size: 1024,
        padding: 1024 / 2,
        power_threshold: 5.0,
        clarity_threshold: 0.7,
    };

    // Signal coming from some source (microphone, generated, etc...)
    let dt = 1.0 / detector_cfg.sample_rate as f32;
    let freq = 300.0;
    let signal: Vec<f32> = (0..detector_cfg.size)
        .map(|x| (2.0 * std::f32::consts::PI * x as f32 * dt * freq).sin())
        .collect();

    let mut detector = McLeodDetector::new(detector_cfg.size, detector_cfg.padding);

    let pitch = my_get_pitch(&signal, &mut detector, &detector_cfg);

    println!("Frequency: {}, Clarity: {}", pitch.frequency, pitch.clarity);
}

fn my_get_pitch<T: pitch_detection::float::Float, U: PitchDetector<T>>(
    signal: &[T],
    detector: &mut U,
    cfg: &DetectorCfg<T>,
) -> Pitch<T> {
    detector
        .get_pitch(
            signal,
            cfg.sample_rate,
            cfg.power_threshold,
            cfg.clarity_threshold,
        )
        .unwrap()
}
