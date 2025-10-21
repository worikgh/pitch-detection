//! The result from note detection.  Adapted from the crate: `pitch-detector`

use core::fmt;
pub const MAX_FREQ: f32 = 1046.50; // C6
pub const MIN_FREQ: f32 = 32.7; // C1
pub const MIN_ZERO_CROSSING_RATE: f32 = 350.; // Hz
pub const A4_FREQ: f32 = 440.0;
pub const NOTES: [&str; 12] = [
    "A", "A#", "B", "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#",
];
// Noticable pitch difference starts at around 10-25 cents
pub const MAX_CENTS_OFFSET: f32 = 10.0;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum NoteName {
    A,
    ASharp,
    B,
    C,
    CSharp,
    D,
    DSharp,
    E,
    F,
    FSharp,
    G,
    GSharp,
}

impl From<&str> for NoteName {
    fn from(s: &str) -> Self {
        match s {
            "A" => NoteName::A,
            "A#" => NoteName::ASharp,
            "B" => NoteName::B,
            "C" => NoteName::C,
            "C#" => NoteName::CSharp,
            "D" => NoteName::D,
            "D#" => NoteName::DSharp,
            "E" => NoteName::E,
            "F" => NoteName::F,
            "F#" => NoteName::FSharp,
            "G" => NoteName::G,
            "G#" => NoteName::GSharp,
            _ => panic!("Invalid pitch"),
        }
    }
}

impl fmt::Display for NoteName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            NoteName::A => write!(f, "A"),
            NoteName::ASharp => write!(f, "A#"),
            NoteName::B => write!(f, "B"),
            NoteName::C => write!(f, "C"),
            NoteName::CSharp => write!(f, "C#"),
            NoteName::D => write!(f, "D"),
            NoteName::DSharp => write!(f, "D#"),
            NoteName::E => write!(f, "E"),
            NoteName::F => write!(f, "F"),
            NoteName::FSharp => write!(f, "F#"),
            NoteName::G => write!(f, "G"),
            NoteName::GSharp => write!(f, "G#"),
        }
    }
}

/// The resut of a pitch detection expressed as a note.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct NoteDetectionResult {
    /// The predominant frequency detected from a signal.
    pub actual_freq: f32,

    /// The note name of the detected note.
    pub note_name: NoteName,

    /// The octave of the detected note.
    pub octave: i32,

    /// The degree to which the detected not is in tune, expressed in
    /// cents. The absolute maximum `cents` is 50.0, since anything
    /// larger than 50 would be considered the next or previous note.
    pub cents: f32,

    /// In `mcleod` detection `clarity` is the maximum volume of the
    /// sample
    pub clarity: f32,

    /// A `NoteDetectionResult` will be marked as `in_tune` if the
    /// `cents` is less than
    /// [`MAX_CENTS_OFFSET`](MAX_CENTS_OFFSET).
    pub in_tune: bool,
}
impl NoteDetectionResult {
    /// Passed the frequency and clarity from note detection.
    pub fn from_freq_clarity(freq: f32, clarity: f32) -> Result<Self, NoteDetectionError> {
        if !(MIN_FREQ..=MAX_FREQ).contains(&freq) {
            return Err(NoteDetectionError::InvalidFrequency(freq));
        }
        // freq / A4_FREQ gives the frequency ratio
        // .log2() converts this to octaves (since each octave doubles the frequency)
        // * 12.0 converts octaves to semitones (12 semitones per octave)
        let steps_from_a4 = (freq / A4_FREQ).log2() * 12.0;
        let steps_from_c5 = steps_from_a4 - 2.0;
        let cents = (steps_from_a4 - steps_from_a4.round()) * 100.0;
        Ok(Self {
            actual_freq: freq,
            note_name: NOTES
                [(steps_from_a4.round() as isize).rem_euclid(NOTES.len() as isize) as usize]
                .into(),
            octave: (5. + (steps_from_c5 / 12.0).floor()) as i32,
            cents,
            in_tune: cents.abs() < MAX_CENTS_OFFSET,
            clarity,
        })
    }
}

/// The errors from note detection
#[derive(Debug)]
pub enum NoteDetectionError {
    InvalidFrequency(f32),
}
impl std::error::Error for NoteDetectionError {}
impl fmt::Display for NoteDetectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoteDetectionError::InvalidFrequency(freq) => {
                write!(f, "NoteDetectionError::InvalidFrequency({freq})")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use float_cmp::ApproxEq;
    #[allow(clippy::too_many_arguments)]
    fn test_pitch_from_f32(
        actual_freq: f32,
        _note_name: NoteName,
        _note_freq: f32,
        octave: i32,
        cents: f32,
        in_tune: bool,
    ) -> Result<(), NoteDetectionError> {
        let pitch = NoteDetectionResult::from_freq_clarity(actual_freq, 1.0)?;
        assert_eq!(
            pitch.octave, octave,
            "Expected octave {}, got {}",
            octave, pitch.octave
        );
        assert!(
            pitch.cents.approx_eq(cents, (0.1, 1)),
            "Expected cents: {}, actual cents: {}",
            cents,
            pitch.cents
        );
        assert_eq!(
            pitch.in_tune, in_tune,
            "Expected in tune {}, got {}",
            in_tune, pitch.in_tune
        );
        Ok(())
    }

    #[test]
    fn pitch_from_f32_works() -> Result<(), NoteDetectionError> {
        test_pitch_from_f32(311.13, NoteName::DSharp, 311.13, 4, 0., true)?;
        test_pitch_from_f32(329.63, NoteName::E, 329.63, 4, 0., true)?;
        test_pitch_from_f32(349.23, NoteName::F, 349.23, 4, 0., true)?;
        test_pitch_from_f32(369.99, NoteName::FSharp, 369.99, 4, 0., true)?;
        test_pitch_from_f32(392., NoteName::G, 392., 4, 0., true)?;
        test_pitch_from_f32(440., NoteName::A, 440., 4, 0., true)?;
        test_pitch_from_f32(493.88, NoteName::B, 493.88, 4, 0., true)?;
        test_pitch_from_f32(523.25, NoteName::C, 523.25, 5, 0., true)?;
        test_pitch_from_f32(880., NoteName::A, 880., 5, 0., true)?;
        test_pitch_from_f32(220., NoteName::A, 220., 3, 0., true)?;
        // Test pitch for a slighly sharp A
        test_pitch_from_f32(448., NoteName::A, 440., 4, 31.194, false)?;
        assert!(test_pitch_from_f32(0., NoteName::A, 0., 0, 0., true).is_err());
        Ok(())
    }
}
