use std::{fmt::Write, io::Write as _};

use cpal::{
    traits::{DeviceTrait, HostTrait},
    SampleRate,
};
use log::warn;
use midir::MidiOutput;

const PREFERRED_SAMPLE_RATE: u32 = 96_000;
const BACKUP_SAMPLE_RATE: u32 = 48_000;

#[derive(Clone)]
pub enum Matcher {
    Index(usize),
    String(String),
}

impl std::str::FromStr for Matcher {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if let Ok(idx) = s.parse() {
            Self::Index(idx)
        } else {
            Self::String(s.to_lowercase())
        })
    }
}

impl Matcher {
    pub fn get<T, E>(
        &self,
        iter: impl IntoIterator<Item = T>,
        accessor: impl Fn(&T) -> Result<String, E>,
    ) -> Result<Option<T>, E> {
        match self {
            Self::Index(idx) => Ok(iter.into_iter().nth(*idx)),
            Self::String(s) => {
                for item in iter {
                    let name = accessor(&item)?;
                    if name.to_lowercase().contains(s) {
                        return Ok(Some(item));
                    }
                }

                Ok(None)
            }
        }
    }
}

#[derive(Debug)]
pub enum MaybeSample<T> {
    Break,
    Sample(T),
}

pub struct NamedFile<S> {
    pub prefix: Option<S>,
    pub pitch: autosam::midi::Pitch,
    pub velocity: Option<u8>,
    pub round_robin: Option<u8>,
}

impl<S> core::fmt::Display for NamedFile<S>
where
    S: AsRef<str>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(p) = &self.prefix {
            f.write_str(p.as_ref())?;
            f.write_char('_')?;
        }

        write!(f, "{}", self.pitch)?;

        if let Some(velocity) = self.velocity {
            write!(f, "_V{velocity}")?;
        }

        if let Some(round_robin) = self.round_robin {
            write!(f, "_RR{}", round_robin + 1)?;
        }

        f.write_str(".wav")
    }
}

pub struct Utf8File(std::fs::File);

impl Utf8File {
    pub fn xml(name: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let mut f = std::fs::File::create(name)?;
        writeln!(f, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
        Ok(Self(f))
    }
}

impl std::fmt::Write for Utf8File {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.write_all(s.as_bytes()).map_err(|_| std::fmt::Error)
    }

    fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> std::fmt::Result {
        self.0.write_fmt(args).map_err(|_| std::fmt::Error)
    }
}

pub fn print_hosts() -> anyhow::Result<()> {
    eprintln!("ID\tName");
    for (id, host) in cpal::available_hosts().into_iter().enumerate() {
        println!("{id}\t{}", host.name());
    }
    Ok(())
}

pub fn print_devices(host: cpal::Host) -> anyhow::Result<()> {
    eprintln!("ID\tIn\tOut\tFs Min\tFs Max\tName");
    for (id, device) in host.devices()?.enumerate() {
        print!("{id}\t");
        print!(
            "{}\t",
            device
                .supported_input_configs()?
                .next()
                .map_or(0, |cfg| cfg.channels())
        );
        print!(
            "{}\t",
            device
                .supported_output_configs()?
                .next()
                .map_or(0, |cfg| cfg.channels())
        );
        if let Some(min_rate) = device
            .supported_input_configs()?
            .chain(device.supported_output_configs()?)
            .map(|cfg| cfg.min_sample_rate().0)
            .min()
        {
            print!("{min_rate:6}\t");
        } else {
            print!("      \t");
        }
        if let Some(max_rate) = device
            .supported_input_configs()?
            .chain(device.supported_output_configs()?)
            .map(|cfg| cfg.max_sample_rate().0)
            .max()
        {
            print!("{max_rate:6}\t");
        } else {
            print!("      \t");
        }
        println!("{}", device.name()?,);
    }
    Ok(())
}

pub fn print_midi_ports(midi_output: MidiOutput) -> anyhow::Result<()> {
    println!("ID\tName");
    for (index, port) in midi_output.ports().into_iter().enumerate() {
        println!("{index}\t{}", midi_output.port_name(&port)?);
    }
    Ok(())
}

pub fn get_best_config(
    input_device: &cpal::Device,
) -> Result<cpal::SupportedStreamConfig, anyhow::Error> {
    let get_config_with_sample_rate = |sr| {
        move |c: cpal::SupportedStreamConfigRange| {
            (c.min_sample_rate().0..=c.max_sample_rate().0)
                .contains(&sr)
                .then(|| c.with_sample_rate(SampleRate(sr)))
        }
    };

    if let Some(c) = input_device
        .supported_input_configs()?
        .find_map(get_config_with_sample_rate(PREFERRED_SAMPLE_RATE))
    {
        return Ok(c);
    }

    warn!("Device does not support preferred sample rate of {PREFERRED_SAMPLE_RATE}");

    if let Some(c) = input_device
        .supported_input_configs()?
        .find_map(get_config_with_sample_rate(BACKUP_SAMPLE_RATE))
    {
        return Ok(c);
    }

    warn!("Device does not support backup sample rate of {BACKUP_SAMPLE_RATE}");

    Ok(input_device.default_input_config()?)
}
