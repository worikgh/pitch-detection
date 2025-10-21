use pitch_detection::note_detection_result::NoteDetectionResult;
use pitch_detection::runner::run;
use std::sync::mpsc;

fn main() {
    let (tx, rx) = mpsc::channel::<NoteDetectionResult>();
    let jh = run(tx, "system:capture_1");
    loop {
        let ndr = match rx.recv() {
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
    _ = jh.join();
}
