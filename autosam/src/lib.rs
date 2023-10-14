//! Utilities and infrastructure for building an auto-sampler
//!
//! # Example
//! ```
//! # use autosam::*;
//! let config = Config { notes: 48..=72, ..Default::default() };
//! let mut sequencer = Sequencer::new(config, 48_000).unwrap();
//!
//! let AdvanceResult::Event { position, note } = sequencer.advance(1) else { panic!() };
//! assert_eq!(position, 0);
//! assert_eq!(note.state(), midi::NoteState::On);
//! assert_eq!(note.pitch().note_number(), 48);
//! assert_eq!(note.velocity(), 127);
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

use core::{num::NonZeroU8, time::Duration};

/// Data types representing MIDI concepts
pub mod midi;
mod tests;

use midi::{InvalidMidiNote, Note, NoteState};

/// Internal utilities for the library
pub mod util {
    /// A generic error for values outside a range of zero to some maximum
    #[derive(Debug)]
    pub struct OutOfBounds<const MAX: u8>(u8);

    impl<const MAX: u8> OutOfBounds<MAX> {
        /// Maximum allowed value
        pub const MAX: u8 = MAX;

        pub(crate) const fn new(value: u8) -> Self {
            Self(value)
        }

        /// Get the value that was larger than the configured limit
        pub const fn value(&self) -> u8 {
            self.0
        }
    }

    impl<const MAX: u8> core::fmt::Display for OutOfBounds<MAX> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "Value {} is larger than maximum {}.", self.0, Self::MAX)
        }
    }

    #[cfg(feature = "std")]
    impl<const MAX: u8> std::error::Error for OutOfBounds<MAX> {}
}

/// Configuration for an autosampling run
#[derive(Debug, Clone)]
pub struct Config {
    /// The range of notes to visit
    ///
    /// Given as MIDI note numbers
    pub notes: core::ops::RangeInclusive<u8>,
    /// The interval (in semitones) to step through the range by
    pub step: NonZeroU8,
    /// The number of velocity levels to sample
    pub velocity_levels: NonZeroU8,
    /// The number of duplicate samples to record at each pitch and velocity
    pub round_robins: NonZeroU8,
    /// The sustain time to hold the note for
    pub length: Duration,
    /// The release time to allow before a new note begins
    pub gap: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            notes: 0..=127,
            step: NonZeroU8::new(1).unwrap(),
            velocity_levels: NonZeroU8::new(1).unwrap(),
            round_robins: NonZeroU8::new(1).unwrap(),
            length: Duration::from_millis(500),
            gap: Duration::from_millis(500),
        }
    }
}

/// An entity that can drive the auto-sampling process
#[derive(Debug)]
pub struct Sequencer {
    length: usize,
    gap: usize,
    pitch: u8,
    pitch_step: u8,
    final_pitch: u8,
    velocity: u8,
    velocity_step: u8,
    round_robin: u8,
    round_robin_count: u8,
    samples_remaining: usize,
    next_status: NoteState,
}

impl Sequencer {
    /// Create a [`Sequencer`] for the provided note distribution at the provided sample rate
    ///
    /// # Errors
    ///
    /// Can return an error if the provided configuration would lead to an invalid state.
    pub fn new(config: Config, sample_rate: u32) -> Result<Self, SequencerError> {
        let Config {
            notes,
            step,
            velocity_levels,
            round_robins,
            length,
            gap,
        } = config;

        let pitch = midi::Pitch::new(*notes.start())
            .map_err(SequencerError::StartNote)?
            .note_number();

        let final_pitch = midi::Pitch::new(*notes.end())
            .map_err(SequencerError::EndNote)?
            .note_number();

        let velocity_levels = velocity_levels.get();
        let velocity_step = (128 + velocity_levels / 2) / velocity_levels;
        if velocity_step == 0 {
            return Err(SequencerError::VelocityLevels(velocity_levels));
        }

        Ok(Self {
            length: ((length * sample_rate).as_millis() / 1_000) as usize,
            gap: ((gap * sample_rate).as_millis() / 1_000) as usize,
            pitch,
            pitch_step: step.get(),
            final_pitch,
            velocity: 127,
            velocity_step,
            round_robin: 0,
            round_robin_count: round_robins.get(),
            samples_remaining: 0,
            next_status: NoteState::On,
        })
    }

    /// Try to move forward, producing any note events that will occur
    ///
    /// If an event is produced, the internal frame counter has only
    /// advanced by its `sample_offset`.
    pub fn advance(&mut self, num_frames: usize) -> AdvanceResult {
        match self.samples_remaining.checked_sub(num_frames) {
            None => {
                let result = AdvanceResult::Event {
                    position: core::mem::take(&mut self.samples_remaining),
                    note: Note {
                        pitch: self.pitch,
                        velocity: self.velocity,
                        state: self.next_status,
                    },
                };

                match self.next_status {
                    // would start a note outside the range
                    NoteState::On if self.pitch > self.final_pitch => {
                        return AdvanceResult::SequenceComplete;
                    }
                    // begin note
                    NoteState::On => {
                        self.samples_remaining = self.length;
                        self.next_status = NoteState::Off;
                    }
                    // end note
                    NoteState::Off => {
                        self.samples_remaining = self.gap;
                        self.next_status = NoteState::On;

                        // prepare state for next note-on
                        self.round_robin += 1;
                        if self.round_robin == self.round_robin_count {
                            self.round_robin = 0;

                            if let Some(next_velocity) =
                                self.velocity.checked_sub(self.velocity_step)
                            {
                                self.velocity = next_velocity;
                            } else {
                                self.velocity = 127;
                                self.pitch += self.pitch_step;
                            }
                        }
                    }
                }

                result
            }
            Some(further) => {
                self.samples_remaining = further;
                AdvanceResult::NoEventsInFrame
            }
        }
    }
}

impl IntoIterator for Sequencer {
    type Item = (usize, Note);
    type IntoIter = SequencerIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        SequencerIntoIter {
            sequencer: self,
            position: 0,
        }
    }
}

/// An iterator that produces all note events in a [`Sequencer`]
///
/// Events will be produced with a corresponding sample position, starting when
/// the sequencer was converted into an iterator and never resetting.
///
/// # Limit behavior
///
/// If the internal position counter reaches [`usize::MAX`], it will wrap, and
/// all subsequently produced events will be produced starting from position 0.
///
/// # Panics
///
/// This struct's [`Iterator::next`] method can panic if the configured
/// note or gap length is [`usize::MAX`] samples at the configured sample rate.
pub struct SequencerIntoIter {
    sequencer: Sequencer,
    position: usize,
}

impl Iterator for SequencerIntoIter {
    type Item = (usize, Note);

    fn next(&mut self) -> Option<Self::Item> {
        match self.sequencer.advance(usize::MAX) {
            AdvanceResult::SequenceComplete => None,
            AdvanceResult::Event { position, note } => {
                self.position = self.position.wrapping_add(position);
                Some((self.position, note))
            }
            AdvanceResult::NoEventsInFrame => {
                unreachable!(
                    "A {} with length {} samples was produced",
                    match self.sequencer.next_status {
                        NoteState::On => "gap",
                        NoteState::Off => "note",
                    },
                    usize::MAX
                )
            }
        }
    }
}

/// The outcome of trying to advance the state of a [`Sequencer`]
#[derive(Debug, PartialEq, Eq)]
pub enum AdvanceResult {
    /// There are no events before the specified position, and the internal counter has been advanced.
    NoEventsInFrame,
    /// An event will occur before the specified time span is complete.
    Event {
        /// The number of frames advanced before reaching this event
        ///
        /// The [`Sequencer`]'s internal state has only been updated to this point.
        position: usize,
        /// The note event
        note: Note,
    },
    /// No more events will be produced by this [`Sequencer`].
    SequenceComplete,
}

/// A problem encountered when creating a [`Sequencer`]
#[derive(Debug)]
pub enum SequencerError {
    /// Invalid start of note range
    StartNote(InvalidMidiNote),
    /// Invalid end of note range
    EndNote(InvalidMidiNote),
    /// Too many velocity levels
    VelocityLevels(u8),
}

impl core::fmt::Display for SequencerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SequencerError::StartNote(e) => write!(f, "Invalid start of note range: {e}"),
            SequencerError::EndNote(e) => write!(f, "Invalid end of note range: {e}"),
            SequencerError::VelocityLevels(n) => {
                write!(f, "Maximum 128 possible velocity layers, specified {n}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SequencerError {}
