#![cfg(test)]

use super::*;

#[test]
fn one_note_sequence() {
    let cfg = Config {
        notes: 60..=60,
        length: Duration::from_millis(100),
        gap: Duration::from_millis(100),
        ..Default::default()
    };

    let mut seq = Sequencer::new(cfg, 1000).unwrap();

    assert_eq!(
        seq.advance(1),
        AdvanceResult::Event {
            position: 0,
            note: Note {
                pitch: 60,
                velocity: 127,
                state: NoteState::On
            }
        }
    );

    assert_eq!(
        seq.advance(101),
        AdvanceResult::Event {
            position: 100,
            note: Note {
                pitch: 60,
                velocity: 127,
                state: NoteState::Off
            }
        }
    );

    assert_eq!(seq.advance(101), AdvanceResult::SequenceComplete);
}

#[test]
fn octave_sequence() {
    let cfg = Config {
        notes: 0..=120,
        step: NonZeroU8::new(12).unwrap(),
        length: Duration::from_millis(100),
        gap: Duration::from_millis(100),
        ..Default::default()
    };

    let mut seq = Sequencer::new(cfg, 1000).unwrap();

    for octave in 0..11 {
        assert_eq!(
            seq.advance(1),
            AdvanceResult::Event {
                position: 0,
                note: Note {
                    pitch: octave * 12,
                    velocity: 127,
                    state: NoteState::On
                }
            }
        );

        assert_eq!(
            seq.advance(101),
            AdvanceResult::Event {
                position: 100,
                note: Note {
                    pitch: octave * 12,
                    velocity: 127,
                    state: NoteState::Off
                }
            }
        );

        assert_eq!(seq.advance(100), AdvanceResult::NoEventsInFrame);
    }

    assert_eq!(seq.advance(101), AdvanceResult::SequenceComplete);
}

#[test]
fn velocity_layer_sequence() {
    let pitch = 60;

    let cfg = Config {
        notes: pitch..=pitch,
        velocity_levels: NonZeroU8::new(5).unwrap(),
        length: Duration::from_millis(100),
        gap: Duration::from_millis(100),
        ..Default::default()
    };

    let mut seq = Sequencer::new(cfg, 1000).unwrap();

    let mut current_velocity = 128;
    for _layer in 0..5 {
        let AdvanceResult::Event {
            position: 0,
            note: Note {
                pitch: actual_pitch,
                velocity,
                state: NoteState::On
            }
        } = seq.advance(1) else {
            panic!("Expected a NoteOn event at position 0, found none.");
        };

        assert_eq!(actual_pitch, pitch);
        assert!(velocity < current_velocity);

        current_velocity = velocity;

        assert_eq!(
            seq.advance(101),
            AdvanceResult::Event {
                position: 100,
                note: Note {
                    pitch,
                    velocity: current_velocity,
                    state: NoteState::Off
                }
            }
        );

        assert_eq!(seq.advance(100), AdvanceResult::NoEventsInFrame);
    }

    assert_eq!(seq.advance(101), AdvanceResult::SequenceComplete);
}

#[test]
fn round_robin_sequence() {
    let pitch = 48;

    let cfg = Config {
        notes: pitch..=pitch,
        round_robins: NonZeroU8::new(4).unwrap(),
        length: Duration::from_millis(100),
        gap: Duration::from_millis(100),
        ..Default::default()
    };

    let mut seq = Sequencer::new(cfg, 1000).unwrap();

    for _round in 0..4 {
        assert_eq!(
            seq.advance(1),
            AdvanceResult::Event {
                position: 0,
                note: Note {
                    pitch,
                    velocity: 127,
                    state: NoteState::On
                }
            }
        );

        assert_eq!(
            seq.advance(101),
            AdvanceResult::Event {
                position: 100,
                note: Note {
                    pitch,
                    velocity: 127,
                    state: NoteState::Off
                }
            }
        );

        assert_eq!(seq.advance(100), AdvanceResult::NoEventsInFrame);
    }

    assert_eq!(seq.advance(101), AdvanceResult::SequenceComplete);
}
