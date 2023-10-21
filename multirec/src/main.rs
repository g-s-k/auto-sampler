use std::{
    num::NonZeroU8,
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use log::{debug, error, info, warn};
use midir::MidiOutput;
use serde::Serialize;

use autosam::{
    midi::{Channel, Note, NoteState, Pitch},
    Config, Sequencer,
};

const ONE: NonZeroU8 = unsafe { NonZeroU8::new_unchecked(1) };

const NOTE_RINGBUFFER_SIZE: usize = 1024;
const AUDIO_RINGBUFFER_SIZE: usize = 4096;

mod arguments;
mod runtime;
mod util;

use arguments::*;
use util::*;

fn main() {
    let args = Args::parse();

    env_logger::Builder::new()
        .filter_level(args.min_log_level)
        .parse_default_env()
        .init();

    if let Err(e) = run(args) {
        error!("Encountered a fatal error: {e}");
    }
}

fn run(args: Args) -> anyhow::Result<()> {
    let host = if let Some(matcher) = args.host {
        cpal::host_from_id(
            matcher
                .get(cpal::available_hosts(), |host| -> anyhow::Result<String> {
                    Ok(host.name().to_string())
                })?
                .ok_or(match matcher {
                    Matcher::Index(i) => RunError::InvalidHostIndex(i),
                    Matcher::String(s) => RunError::NoSuchHost(s),
                })?,
        )?
    } else {
        cpal::default_host()
    };

    let midi_output = MidiOutput::new("MIDI Output")?;

    let mut output_dir = std::env::current_dir()?;
    let mut file_name_prefix = None;
    let mut output_format = arguments::OutputFormat::Raw;
    let is_dry_run;
    let config;
    let should_save;
    let should_trim;

    match args.cmd {
        Command::Show(Show::AudioHosts) => {
            return print_hosts();
        }
        Command::Show(Show::AudioDevices) => {
            return print_devices(host);
        }
        Command::Show(Show::MidiPorts) => {
            return print_midi_ports(midi_output);
        }
        Command::Test {
            dry_run,
            note,
            timing,
        } => {
            is_dry_run = dry_run;
            let length = Duration::from_secs_f64(timing.sustain);
            let gap = Duration::from_secs_f64(timing.release);

            info!("Testing note {note} with sustain time {length:?} and release time {gap:?}");

            should_save = false;
            should_trim = false;
            config = Config {
                notes: note.note_number()..=note.note_number(),
                step: ONE,
                velocity_levels: ONE,
                round_robins: ONE,
                length,
                gap,
            };
        }
        Command::Run {
            dry_run,
            start,
            end,
            step,
            velocity_layers,
            round_robins,
            trim_start,
            timing,
            output_directory,
            file_prefix,
            format,
        } => {
            is_dry_run = dry_run;
            let length = Duration::from_secs_f64(timing.sustain);
            let gap = Duration::from_secs_f64(timing.release);

            output_format = format;
            file_name_prefix = file_prefix;
            if let Some(d) = output_directory {
                output_dir = d;
            }

            info!(
                "Recording every {} from {start} until {end} \
                with {velocity_layers} velocity layer{}{}, \
                sustain time {length:?} and release time {gap:?}",
                if step.get() == 1 {
                    "note".to_string()
                } else {
                    format!("{step} notes")
                },
                if velocity_layers.get() == 1 { "" } else { "s" },
                if round_robins.get() == 1 {
                    String::new()
                } else {
                    format!(" and {round_robins} round-robin variations")
                },
            );

            should_save = true;
            should_trim = trim_start;
            config = Config {
                notes: start.note_number()..=end.note_number(),
                step,
                velocity_levels: velocity_layers,
                round_robins,
                length: Duration::from_secs_f64(timing.sustain),
                gap: Duration::from_secs_f64(timing.release),
            };
        }
    }

    let input_device = if let Some(matcher) = args.input_device {
        matcher
            .get(host.input_devices()?, |d| d.name())?
            .ok_or(match matcher {
                Matcher::Index(i) => RunError::InvalidDeviceIndex(i),
                Matcher::String(s) => RunError::NoSuchDevice(s),
            })?
    } else {
        host.default_input_device()
            .ok_or(RunError::NoDefaultInputDevice)?
    };
    info!("Using audio input device {}", input_device.name()?);

    let supported_input_config = get_best_config(&input_device)?;
    info!(
        "Sample rate set to {}",
        supported_input_config.sample_rate().0
    );

    let mut input_config = supported_input_config.config();
    input_config.buffer_size = match supported_input_config.buffer_size() {
        cpal::SupportedBufferSize::Range { min, max } => {
            let buffer_size = min.next_power_of_two().clamp(32, *max);
            info!("Buffer size set to {buffer_size}");
            cpal::BufferSize::Fixed(buffer_size)
        }
        cpal::SupportedBufferSize::Unknown => {
            warn!("Audio device did not report a buffer size, using the default");
            cpal::BufferSize::Default
        }
    };
    input_config.channels = input_config.channels.min(2);
    info!("Channels set to {}", input_config.channels);

    let state = Arc::new(runtime::RunState::new(*config.notes.start()));

    let round_robins = config.round_robins.get();
    let velocity_levels = config.velocity_levels.get();

    let seq = Sequencer::new(config, input_config.sample_rate.0)?;
    let channel = Channel::new(args.midi_channel.get() - 1)?;

    if is_dry_run {
        eprintln!("Sample Offset       \tEvent\tPitch\tVelo\tMIDI");
        eprintln!("--------------------\t-----\t-----\t----\t----");

        for (sample_offset, event) in seq {
            println!(
                "{sample_offset:20}\t{}\t{:5}\t{:4}\t{:?}",
                if event.state() == NoteState::On {
                    "On"
                } else {
                    "Off"
                },
                event.pitch(),
                event.velocity(),
                event.as_midi_message(channel),
            );
        }

        return Ok(());
    }

    let (note_tx, mut note_rx) = rtrb::RingBuffer::<Note>::new(NOTE_RINGBUFFER_SIZE);
    let (audio_tx, mut audio_rx) = rtrb::RingBuffer::new(AUDIO_RINGBUFFER_SIZE);

    let has_vel = velocity_levels > 1;
    let has_rr = round_robins > 1;

    let entries = std::thread::scope(|scope| {
        let output_dir = &output_dir;
        let file_name_prefix = &file_name_prefix;

        let player_handle = std::thread::Builder::new()
            .name("midi-output".into())
            .spawn_scoped(scope, {
                let state = state.clone();

                let midi_ports = midi_output.ports();
                let midi_out_port = args
                    .midi_port
                    .get(&midi_ports, |p| midi_output.port_name(p))?
                    .ok_or(match args.midi_port {
                        Matcher::Index(i) => RunError::InvalidPortIndex(i),
                        Matcher::String(s) => RunError::NoSuchPort(s),
                    })?;
                let port_name = midi_output.port_name(midi_out_port)?;
                let mut midi_connection = midi_output
                    .connect(midi_out_port, "autosam")
                    .expect("Failed to connect to selected MIDI port");

                info!("Connected to MIDI output port {port_name}");

                midi_connection.send(&channel.all_sound_off())?;

                move || {
                    while {
                        let is_abandoned = note_rx.is_abandoned();
                        let sequence_is_done = state.done();

                        if is_abandoned {
                            debug!("MIDI producer was dropped");
                        }

                        if sequence_is_done {
                            debug!("Audio callback has set `done` flag to `true`");
                        }

                        !is_abandoned && !sequence_is_done
                    } {
                        let mut any_messages = false;

                        'notes: loop {
                            match note_rx.pop() {
                                Err(rtrb::PopError::Empty) => break 'notes,
                                Ok(note) => {
                                    any_messages = true;
                                    let msg = note.as_midi_message(channel);
                                    debug!("Sending note {msg:?}");
                                    if let Err(e) = midi_connection.send(&msg) {
                                        error!("Failed to send MIDI note on message: {e}");
                                    }
                                }
                            }
                        }

                        if !any_messages {
                            std::thread::sleep(Duration::from_millis(1));
                        }
                    }
                }
            })?;

        let writer_builder = std::thread::Builder::new().name("wav-writer".into());

        let writer_handle = if should_save {
            let spec = hound::WavSpec {
                channels: input_config.channels,
                sample_rate: input_config.sample_rate.0,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };

            if !output_dir.exists() {
                std::fs::create_dir_all(output_dir)?;
            }

            let state = state.clone();

            writer_builder.spawn_scoped(scope, move || -> anyhow::Result<Vec<_>> {
                let mut entries = Vec::new();

                let mut create_file_name = || -> anyhow::Result<PathBuf> {
                    let (pitch, velocity, round_robin) = state.note(Ordering::Acquire);

                    let entry = util::NamedFile {
                        prefix: file_name_prefix.as_ref(),
                        pitch: Pitch::new(pitch)?,
                        velocity: has_vel.then_some(velocity),
                        round_robin: has_rr.then_some(round_robin),
                    };

                    let path = output_dir.join(format!("{entry}"));
                    entries.push(entry);

                    Ok(path)
                };

                let mut writer = hound::WavWriter::create(create_file_name()?, spec)?;

                // wait for first note event to start writing
                loop {
                    match audio_rx.pop() {
                        Err(rtrb::PopError::Empty) if state.done() => {
                            debug!(
                            "Audio callback set `done` flag to `true` before any data was recorded"
                        );
                            return Ok(entries);
                        }
                        Err(rtrb::PopError::Empty) => {
                            std::thread::sleep(Duration::from_millis(1));
                        }
                        Ok(MaybeSample::Break) => break,
                        _ => {}
                    }
                }

                loop {
                    match audio_rx.pop() {
                        Err(rtrb::PopError::Empty) if state.done() => {
                            debug!("I/O thread shutting down");
                            writer.finalize()?;
                            return Ok(entries);
                        }
                        Err(rtrb::PopError::Empty) => {
                            std::thread::sleep(Duration::from_millis(1));
                        }
                        Ok(MaybeSample::Break) => {
                            writer.finalize()?;
                            debug!("Creating next WAV file");
                            writer = hound::WavWriter::create(create_file_name()?, spec)?;
                        }
                        Ok(MaybeSample::Sample(data)) => {
                            writer.write_sample(data)?;
                        }
                    }
                }
            })
        } else {
            let state = state.clone();

            writer_builder.spawn_scoped(scope, move || loop {
                match audio_rx.pop() {
                    Err(rtrb::PopError::Empty) if state.done() => {
                        debug!("I/O thread shutting down");
                        return Ok(Vec::new());
                    }
                    Err(rtrb::PopError::Empty) => {
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    Ok(MaybeSample::Break) | Ok(MaybeSample::Sample(_)) => {
                        // do nothing
                    }
                }
            })
        }?;

        let mut processor = runtime::AudioProcessor {
            seq,
            sender: note_tx,
            writer: audio_tx,
            channels: usize::from(input_config.channels),
            state: state.clone(),
            latency_timer: None,
            trim_start: should_trim,
        };

        let err_fn = |e| {
            error!("Encountered an error while processing input audio: {e}");
        };

        let stream = match supported_input_config.sample_format() {
            cpal::SampleFormat::I8 => {
                info!("Incoming sample format is 8 bit signed");
                input_device.build_input_stream(
                    &input_config,
                    move |data, _: &_| processor.write_input_data::<i8>(data),
                    err_fn,
                    None,
                )?
            }
            cpal::SampleFormat::I16 => {
                info!("Incoming sample format is 16 bit signed");
                input_device.build_input_stream(
                    &input_config,
                    move |data, _: &_| processor.write_input_data::<i16>(data),
                    err_fn,
                    None,
                )?
            }
            cpal::SampleFormat::I32 => {
                info!("Incoming sample format is 32 bit signed");
                input_device.build_input_stream(
                    &input_config,
                    move |data, _: &_| processor.write_input_data::<i32>(data),
                    err_fn,
                    None,
                )?
            }
            cpal::SampleFormat::F32 => {
                info!("Incoming sample format is 32 bit float");
                input_device.build_input_stream(
                    &input_config,
                    move |data, _: &_| processor.write_input_data::<f32>(data),
                    err_fn,
                    None,
                )?
            }
            sample_format => {
                return Err(anyhow::Error::msg(format!(
                    "Unsupported sample format '{sample_format}'"
                )))
            }
        };

        debug!("Capturing input");

        stream.play()?;

        debug!("Waiting for MIDI thread to finish");

        player_handle
            .join()
            .map_err(|e| RunError::MidiPanic(format!("{e:?}")))?;

        debug!("MIDI player exited, waiting for WAV writer");

        let entries = writer_handle
            .join()
            .map_err(|e| RunError::IoPanic(format!("{e:?}")))??;

        debug!("WAV writer exited");

        drop(stream);

        Ok(entries)
    })?;

    let latency = state.latency();
    let latency_text = format!(
        "Approximate latency: {:?} ({latency} samples)",
        Duration::from_millis(latency as u64 * 1_000) / input_config.sample_rate.0
    );

    if should_save {
        info!("Recordings complete");
        if latency != 0 {
            info!("{latency_text}");
        }

        let mut zip_compression = None;
        let mut zipped_name = output_dir.with_extension("zip");

        match output_format {
            OutputFormat::Raw => {} // do nothing
            OutputFormat::Zip => {
                zip_compression = Some(zip::CompressionMethod::Deflated);
            }
            OutputFormat::Bitwig => {
                zip_compression = Some(zip::CompressionMethod::Stored);
                zipped_name = output_dir.with_extension("multisample");

                let mut multi = dot_multisample::Multisample::default()
                    .with_generator("multirec")
                    .with_samples(entries.iter().enumerate().map(|(idx, f)| {
                        let note = f.pitch.note_number();
                        let mut key = dot_multisample::Key::default().with_root(note);

                        if let Some(prev_note) = entries[..idx]
                            .iter()
                            .map(|f| f.pitch.note_number())
                            .rfind(|n| n < &note)
                        {
                            let middle = (note - prev_note) / 2 + prev_note;
                            key = key.with_low(middle);
                        }

                        if let Some(next_note) = entries[idx..]
                            .iter()
                            .map(|f| f.pitch.note_number())
                            .find(|n| n > &note)
                        {
                            let middle =
                                ((next_note - note) / 2 + note).saturating_sub(1).max(note);
                            key = key.with_high(middle);
                        }

                        let vel = f.velocity;
                        let velocity = entries[idx..]
                            .iter()
                            .map(|f| f.velocity)
                            .find(|v| v < &vel)
                            .flatten()
                            .map(|next_vel| {
                                dot_multisample::ZoneInfo::default()
                                    .with_high(vel)
                                    .with_low(next_vel + 1)
                            });

                        dot_multisample::Sample::default()
                            .with_file(std::path::PathBuf::from(format!("{f}")))
                            .with_key(key)
                            .with_velocity(velocity)
                            .with_zone_logic(dot_multisample::ZoneLogic::RoundRobin)
                    }));

                if let Some(p) = &file_name_prefix {
                    multi = multi.with_name(p);
                }

                let mut manifest_file = util::Utf8File::xml(output_dir.join("multisample.xml"))?;
                let mut ser = quick_xml::se::Serializer::new(&mut manifest_file);
                ser.indent('\t', 1);
                multi.serialize(ser)?;
            }
        }

        if let Some(compression) = zip_compression {
            let zipped = std::fs::File::create(zipped_name)?;
            let mut zip_writer = zip::ZipWriter::new(&zipped);

            let opts = zip::write::FileOptions::default().compression_method(compression);

            for file in output_dir.read_dir()? {
                let file = file?;

                if file.path().is_file() {
                    zip_writer.start_file(file.file_name().to_string_lossy(), opts)?;
                    std::io::copy(&mut std::fs::File::open(file.path())?, &mut zip_writer)?;
                }
            }

            zip_writer.finish()?;
            std::fs::remove_dir_all(output_dir)?;
        }
    } else {
        info!("Test complete");
        if latency != 0 {
            println!("{latency_text}");
        }
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum RunError {
    #[error("Selected audio host ID ({0}) does not exist")]
    InvalidHostIndex(usize),
    #[error("No audio host found with name like `{0}`")]
    NoSuchHost(String),
    #[error("Selected audio device ID ({0}) does not exist")]
    InvalidDeviceIndex(usize),
    #[error("No audio device found with name like `{0}`")]
    NoSuchDevice(String),
    #[error("No default input device was found")]
    NoDefaultInputDevice,
    #[error("Selected MIDI port ID ({0}) does not exist")]
    InvalidPortIndex(usize),
    #[error("No MIDI port found with name like `{0}`")]
    NoSuchPort(String),
    #[error("MIDI thread panicked: {0}")]
    MidiPanic(String),
    #[error("I/O thread panicked: {0}")]
    IoPanic(String),
}
