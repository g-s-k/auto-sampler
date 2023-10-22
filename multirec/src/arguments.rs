use std::{num::NonZeroU8, path::PathBuf};

use clap::Parser;

use autosam::midi::Pitch;

use crate::{util::Matcher, ONE};

#[derive(Parser)]
#[command(author, version, about)]
pub struct Args {
    #[clap(subcommand)]
    pub cmd: Command,
    /// Select an audio host by index
    #[arg(long)]
    pub host: Option<Matcher>,
    /// Select an audio input to record from
    #[arg(long, short = 'i')]
    pub input_device: Option<Matcher>,
    /// Select a MIDI port to output to
    #[arg(long, default_value = "0")]
    pub midi_port: Matcher,
    /// Select a MIDI channel to send on
    #[arg(long, short = 'c', default_value_t = ONE)]
    pub midi_channel: NonZeroU8,
    /// Specify verbosity of log messages
    #[arg(long, default_value = "warn")]
    pub min_log_level: log::LevelFilter,
}

#[derive(clap::Subcommand)]
pub enum Command {
    /// Display information about the system
    #[clap(subcommand)]
    Show(Show),
    /// Run the auto-sampling routine
    Run {
        /// Multi-sample package format to generate
        #[arg(long, short = 'f', default_value = "raw")]
        format: OutputFormat,
        /// Directory to save recordings in [default: current]
        #[arg(long, short = 'o')]
        output_directory: Option<PathBuf>,
        /// Prefix for file names
        #[arg(long, short = 'p')]
        file_prefix: Option<String>,
        /// Print configuration and exit
        #[clap(long, short = 'n')]
        dry_run: bool,
        /// Lowest note to sample (MIDI note name or number)
        #[arg(long, default_value = "21")]
        start: Pitch,
        /// Highest note to sample (MIDI note name or number)
        #[arg(long, default_value = "108")]
        end: Pitch,
        /// Step between notes, in semitones
        #[arg(long, default_value_t = ONE)]
        step: NonZeroU8,
        /// Number of velocity layers to sample
        #[arg(long, default_value_t = ONE)]
        velocity_layers: NonZeroU8,
        /// Number of round-robin samples to take of each velocity layer
        #[arg(long, default_value_t = ONE)]
        round_robins: NonZeroU8,
        /// Discard silence at the beginning of each sample
        #[arg(long)]
        trim_start: bool,
        #[clap(flatten)]
        timing: Timing,
    },
    /// Play a single note to check routing configuration
    Test {
        /// Print configuration and exit
        #[clap(long, short = 'n')]
        dry_run: bool,
        /// Note to test (MIDI note name or number)
        #[arg(long, default_value = "48")]
        note: Pitch,
        #[clap(flatten)]
        timing: Timing,
    },
}

#[derive(clap::Subcommand)]
pub enum Show {
    /// List available audio hosts (drivers)
    AudioHosts,
    /// List available audio devices for the selected host
    AudioDevices,
    /// List available MIDI ports
    MidiPorts,
}

#[derive(Parser)]
pub struct Timing {
    /// Length of each note before sending NoteOff message, in seconds
    #[arg(long, default_value_t = 1.0)]
    pub sustain: f64,
    /// Time to wait after NoteOff before starting next note, in seconds
    #[arg(long, default_value_t = 0.5)]
    pub release: f64,
}

#[derive(Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Raw,
    Zip,
    Sfz,
    Bitwig,
}
