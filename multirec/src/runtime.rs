use std::sync::{
    atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering},
    Arc,
};

use cpal::FromSample;
use log::error;

use autosam::{
    midi::{Note, NoteState},
    AdvanceResult, Sequencer,
};

use crate::util::MaybeSample;

pub struct RunState {
    note_data: AtomicU32,
    done: AtomicBool,
    latency: AtomicUsize,
}

impl RunState {
    pub fn new(initial_pitch: u8) -> Self {
        Self {
            note_data: AtomicU32::new(u32::from_be_bytes([1, initial_pitch, 127, 0])),
            done: AtomicBool::new(false),
            latency: AtomicUsize::new(0),
        }
    }

    pub fn done(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }

    pub fn latency(&self) -> usize {
        self.latency.load(Ordering::Acquire)
    }

    pub fn note(&self, ordering: Ordering) -> (u8, u8, u8) {
        let [_, note, velocity, round_robin] = self.note_data.load(ordering).to_be_bytes();
        (note, velocity, round_robin)
    }

    pub fn new_note(&self, note: &Note) {
        let [first, old_pitch, old_velocity, old_robin] =
            self.note_data.load(Ordering::Relaxed).to_be_bytes();

        let pitch = note.pitch().note_number();
        let velocity = note.velocity();

        self.note_data.store(
            u32::from_be_bytes([
                0,
                pitch,
                velocity,
                if first == 0 && old_pitch == pitch && old_velocity == velocity {
                    old_robin + 1
                } else {
                    0
                },
            ]),
            Ordering::Release,
        );
    }
}

pub struct AudioProcessor<U> {
    pub seq: Sequencer,
    pub sender: rtrb::Producer<Note>,
    pub writer: rtrb::Producer<MaybeSample<U>>,
    pub channels: usize,
    pub state: Arc<RunState>,
    pub latency_timer: Option<usize>,
    pub trim_start: bool,
}

impl AudioProcessor<i16> {
    pub fn write_input_data<T>(&mut self, input: &[T])
    where
        T: cpal::Sample,
        i16: FromSample<T>,
    {
        for frame in input.chunks(self.channels) {
            if let Some(t) = &mut self.latency_timer {
                *t += 1;
            }

            match self.seq.advance(1) {
                AdvanceResult::NoEventsInFrame => {}
                AdvanceResult::SequenceComplete => {
                    self.state.done.store(true, Ordering::Release);
                }
                AdvanceResult::Event { position: _, note } => {
                    if let NoteState::On = note.state() {
                        self.latency_timer = Some(0);
                        self.state.new_note(&note);

                        if let Err(e) = self.writer.push(MaybeSample::Break) {
                            error!("Out of capacity in I/O buffer [{}]: {e}", line!());
                        }
                    }

                    if let Err(e) = self.sender.push(note) {
                        error!("Out of capacity in event buffer: {e}");
                    }
                }
            }

            if frame.iter().all(|s| i16::from_sample_(*s) == 0i16) {
                if self.trim_start {
                    continue;
                }
            } else if let Some(t) = self.latency_timer.take() {
                self.state.latency.fetch_max(t, Ordering::Release);
            }

            for sample in frame {
                if let Err(e) = self
                    .writer
                    .push(MaybeSample::Sample(i16::from_sample_(*sample)))
                {
                    error!("Out of capacity in I/O buffer [{}]: {e}", line!());
                }
            }
        }
    }
}
