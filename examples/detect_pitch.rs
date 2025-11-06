use pitch_detection::note_detection_result::NoteDetectionResult;
use pitch_detection::runner::{pitch_detection_run, start_jack, Detector, DetectorCfg};
use std::sync::{mpsc, Arc, Mutex};

fn main() {
    let (tx_f32, rx_f32) = mpsc::channel::<f32>();
    let (tx_ndr, rx_ndr) = mpsc::channel::<NoteDetectionResult>();

    let ac = match start_jack(tx_f32, "system:capture_1") {
        Ok(ac) => ac,
        Err(err) => panic!("Error detect_pitch: Failed to start jack for pitch detection: {err}"),
    };

    let kill_switch = Arc::new(Mutex::new(false));
    // Results from `examples/tester.rs` for quite distorted guitar samples:
    // |actual_note | frequency| match|
    // |:-----------|---------:|-----:|
    // |E/2         |        13|  0.77|
    // |A/2         |        13|  0.92|
    // |D/3         |        12|  1.00|
    // |G/3         |        11|  1.00|
    // |B/3         |        14|  0.14|
    // |E/4         |        14|  0.64|
    //
    // Minimum: 0.14  Mean: 0.75
    // Info: McLeod size/4096 power/10 clarity/0.5 padding/512
    let detector_cfg = DetectorCfg {
        sample_rate: ac.as_client().sample_rate() as u32,
        size: 4096,
        padding: 512,
        power_threshold: 10.0,
        clarity_threshold: 0.5,
        detector: Detector::McLeod,
    };
    let jh = pitch_detection_run(tx_ndr, rx_f32, &detector_cfg, Some(kill_switch.clone()));
    loop {
        let ndr = match rx_ndr.recv() {
            Ok(ndr) => ndr,
            Err(err) => {
                eprintln!("DBG detect_pitch: Main loop failed with error: {err}");
                break;
            }
        };

        // : >+5.2:
        //     >: Aligns the output to the right.
        //     -: Shows a minus sign for negative values
        //     6: Minimum width of the output, including the sign and decimal point.
        //     .2: Specifies that there should be two digits after the decimal point.

        println!(
            "Pitch Detection: Note: {} Octave: {}  cents: {: >-6.2} clarity: {:0.2}",
            ndr.note_name, ndr.octave, ndr.cents, ndr.clarity,
        );
    }
    _ = ac.deactivate();
    _ = jh.join();
}
