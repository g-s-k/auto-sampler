/// A note event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Note {
    /// Pitch (as MIDI note number)
    pub(crate) pitch: u8,
    /// Velocity (up to 127)
    pub(crate) velocity: u8,
    /// Event type
    pub(crate) state: NoteState,
}

impl Note {
    /// Format as a 3-byte MIDI message
    pub fn as_midi_message(&self, channel: Channel) -> [u8; 3] {
        [
            self.state.as_midi_message(channel),
            self.pitch,
            self.velocity,
        ]
    }

    /// Get the pitch of the note
    pub fn pitch(&self) -> Pitch {
        Pitch(self.pitch)
    }

    /// Get the velocity of the note
    pub fn velocity(&self) -> u8 {
        self.velocity
    }

    /// Get the note state (on or off) after this event
    pub fn state(&self) -> NoteState {
        self.state
    }
}

/// Type of note event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteState {
    /// NoteOn (`0x90`)
    On,
    /// NoteOff (`0x80`)
    Off,
}

impl NoteState {
    /// Format as the first byte of a MIDI note message
    pub fn as_midi_message(&self, channel: Channel) -> u8 {
        channel.0
            | match self {
                Self::On => 0x90,
                Self::Off => 0x80,
            }
    }
}

/// A MIDI channel
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Channel(u8);

impl Channel {
    /// Get an instance of [`Channel`]
    pub fn new(channel: u8) -> Result<Self, InvalidMidiChannel> {
        if channel > 15 {
            return Err(InvalidMidiChannel::new(channel));
        }

        Ok(Self(channel))
    }

    /// The MIDI channel number (zero based)
    pub fn number(&self) -> u8 {
        self.0
    }

    /// Produce a MIDI "All Sound Off" message on this instance's channel
    pub fn all_sound_off(&self) -> [u8; 3] {
        [0xB0 | self.0, 120, 0]
    }
}

/// A MIDI channel greater than 15 was provided
pub type InvalidMidiChannel = crate::util::OutOfBounds<15>;

/// A MIDI note number greater than 127 was provided
pub type InvalidMidiNote = crate::util::OutOfBounds<127>;

/// A MIDI pitch value
///
/// Implements [`Display`] as its note name.
///
/// # Example
///
/// ```
/// let note = autosam::midi::Pitch::new(60).unwrap();
/// assert_eq!(format!("{note}"), "C4");
/// ```
///
/// [`Display`]: core::fmt::Display
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pitch(u8);

impl Pitch {
    /// Create and validate a MIDI pitch value
    pub const fn new(note_number: u8) -> Result<Self, InvalidMidiNote> {
        if note_number > 127 {
            return Err(InvalidMidiNote::new(note_number));
        }

        Ok(Self(note_number))
    }

    /// Get the inner pitch value
    pub fn note_number(&self) -> u8 {
        self.0
    }
}

impl core::fmt::Display for Pitch {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        const NAMES: [&str; 12] = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];

        let note = self.0 % 12;
        let octave = (self.0 / 12) as i8 - 1;

        write!(f, "{}{octave}", NAMES[note as usize])
    }
}

impl core::str::FromStr for Pitch {
    type Err = ParsePitchError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(if let Ok(note_number) = s.parse() {
            note_number
        } else {
            let mut octave_start = 1;
            let mut chars = s.chars();

            let note_name = chars.next().ok_or(Self::Err::Empty)?;
            let (mut note, can_sharpen) = match note_name.to_ascii_uppercase() {
                'C' => (0, true),
                'D' => (2, true),
                'E' => (4, false),
                'F' => (5, true),
                'G' => (7, true),
                'A' => (9, true),
                'B' => (11, false),
                _ => return Err(Self::Err::InvalidNoteName(note_name)),
            };

            if let Some('#') = chars.next() {
                if !can_sharpen {
                    return Err(Self::Err::InvalidSharp(note_name.to_ascii_uppercase()));
                }

                note += 1;
                octave_start += 1;
            }

            let octave: i8 = s[octave_start..].parse().map_err(Self::Err::OctaveText)?;
            let octave: u8 = (octave + 1).try_into().map_err(Self::Err::OctaveNumber)?;

            octave * 12 + note
        })
        .map_err(Self::Err::OutOfRange)
    }
}

/// Invalid text specifying a MIDI pitch
#[derive(Debug)]
pub enum ParsePitchError {
    /// Empty string
    Empty,
    /// Does not start with a number or A through G
    InvalidNoteName(char),
    /// Note that can't be sharpened
    InvalidSharp(char),
    /// Does not have a number after the letter (or sharp marker)
    OctaveText(core::num::ParseIntError),
    /// Octave number is less than -1
    OctaveNumber(core::num::TryFromIntError),
    /// Note is larger than 127
    OutOfRange(InvalidMidiNote),
}

impl core::fmt::Display for ParsePitchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParsePitchError::Empty => write!(f, "Provided string was empty"),
            ParsePitchError::InvalidNoteName(c) => write!(f, "{c} is not a valid note name"),
            ParsePitchError::InvalidSharp(c) => {
                write!(f, "Note {c} cannot have a sharp attached to it")
            }
            ParsePitchError::OctaveText(e) => write!(f, "Failed to parse octave number: {e}"),
            ParsePitchError::OctaveNumber(e) => {
                write!(
                    f,
                    "Could not convert octave number into unsigned integer: {e}"
                )
            }
            ParsePitchError::OutOfRange(e) => {
                write!(f, "{e}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParsePitchError {}
