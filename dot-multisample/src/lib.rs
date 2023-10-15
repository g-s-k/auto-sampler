//! .multisample format domain model
//!
//! Matches schema on [GitHub](https://github.com/bitwig/multisample) as of commit `4e7971f1`.
//!
//! ## Example
//!
//! ```
//! # use std::path::Path;
//! # use dot_multisample::*;
//! # let path = std::env::current_dir().unwrap();
//! let multi = Multisample::default()
//!     .with_name("My Instrument")
//!     .with_generator("Rust")
//!     .with_category("Piano")
//!     .with_creator("Me")
//!     .with_description("Toy piano I found at the second hand shop")
//!     .with_keywords(["noisy", "dirty", "metallic"])
//!     .with_samples([
//!         Sample::default()
//!             .with_file(path.join("C2.wav"))
//!             .with_key(Key::default().with_root(36)),
//!         Sample::default()
//!             .with_file(path.join("C3.wav"))
//!             .with_key(Key::default().with_root(48)),
//!         Sample::default()
//!             .with_file(path.join("C4.wav"))
//!             .with_key(Key::default().with_root(60)),
//!     ]);
//! ```

#![warn(missing_docs)]

use std::borrow::Cow;

/// A multi-sample mapping for an instrument
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename = "multisample")]
pub struct Multisample<'a> {
    #[serde(
        borrow,
        default,
        rename = "@name",
        skip_serializing_if = "str::is_empty"
    )]
    name: Cow<'a, str>,
    #[serde(borrow, default, skip_serializing_if = "str::is_empty")]
    generator: Cow<'a, str>,
    #[serde(borrow, default, skip_serializing_if = "str::is_empty")]
    category: Cow<'a, str>,
    #[serde(borrow, default, skip_serializing_if = "str::is_empty")]
    creator: Cow<'a, str>,
    #[serde(borrow, default, skip_serializing_if = "str::is_empty")]
    description: Cow<'a, str>,
    #[serde(borrow, default, skip_serializing_if = "Keywords::is_empty")]
    keywords: Keywords<'a>,
    #[serde(borrow, default, rename = "group")]
    groups: Cow<'a, [Group<'a>]>,
    #[serde(borrow, default, rename = "sample")]
    samples: Cow<'a, [Sample<'a>]>,
}

impl<'a> Multisample<'a> {
    /// Clones any borrowed data and returns a copy with a `'static` lifetime
    pub fn to_owned(self) -> Multisample<'static> {
        Multisample {
            name: Cow::Owned(self.name.into_owned()),
            generator: Cow::Owned(self.generator.into_owned()),
            category: Cow::Owned(self.category.into_owned()),
            creator: Cow::Owned(self.creator.into_owned()),
            description: Cow::Owned(self.description.into_owned()),
            keywords: Keywords {
                list: self
                    .keywords
                    .list
                    .iter()
                    .map(|s| Cow::Owned(s.to_string()))
                    .collect(),
            },
            groups: self
                .groups
                .iter()
                .map(|g| Group {
                    name: Cow::Owned(g.name.to_string()),
                    color: g.color,
                })
                .collect(),
            samples: self
                .samples
                .iter()
                .map(|s| Sample {
                    file: Cow::Owned(s.file.to_path_buf()),
                    ..s.clone()
                })
                .collect(),
        }
    }

    /// Set the name of the multi-sampled instrument
    pub fn with_name(self, name: impl Into<Cow<'a, str>>) -> Self {
        Self {
            name: name.into(),
            ..self
        }
    }

    /// Set the name of the software tool generating the mapping
    pub fn with_generator(self, generator: impl Into<Cow<'a, str>>) -> Self {
        Self {
            generator: generator.into(),
            ..self
        }
    }

    /// Set the general kind of instrument this is
    pub fn with_category(self, category: impl Into<Cow<'a, str>>) -> Self {
        Self {
            category: category.into(),
            ..self
        }
    }

    /// Set the user who is creating the mapping
    pub fn with_creator(self, creator: impl Into<Cow<'a, str>>) -> Self {
        Self {
            creator: creator.into(),
            ..self
        }
    }

    /// Provide a longer-form text description of the instrument
    pub fn with_description(self, description: impl Into<Cow<'a, str>>) -> Self {
        Self {
            description: description.into(),
            ..self
        }
    }

    /// Set the keywords associated with this instrument
    pub fn with_keywords<S: Into<Cow<'a, str>>>(
        self,
        keywords: impl IntoIterator<Item = S>,
    ) -> Self {
        Self {
            keywords: Keywords {
                list: keywords.into_iter().map(Into::into).collect(),
            },
            ..self
        }
    }

    /// Set the list of sample groups
    pub fn with_groups(self, groups: impl IntoIterator<Item = Group<'a>>) -> Self {
        Self {
            groups: groups.into_iter().collect(),
            ..self
        }
    }

    /// Set the list of sample mappings
    pub fn with_samples(self, samples: impl IntoIterator<Item = Sample<'a>>) -> Self {
        Self {
            samples: samples.into_iter().collect(),
            ..self
        }
    }

    /// Name of the multi-sampled instrument
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Name of the software tool generating the mapping
    pub fn generator(&self) -> &str {
        &self.generator
    }

    /// General kind of instrument
    pub fn category(&self) -> &str {
        &self.category
    }

    /// User who created the mapping
    pub fn creator(&self) -> &str {
        &self.creator
    }

    /// Longer-form text description of the instrument
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Keywords to aid in finding and organizing instruments
    pub fn keywords(&self) -> &[Cow<'a, str>] {
        &self.keywords.list
    }

    /// Groups that can be referenced from the sample list
    pub fn groups(&self) -> &[Group] {
        &self.groups
    }

    /// Sample mappings in this instrument
    pub fn samples(&self) -> &[Sample] {
        &self.samples
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Keywords<'a> {
    #[serde(borrow, default, rename = "keyword")]
    list: Cow<'a, [Cow<'a, str>]>,
}

impl Keywords<'_> {
    fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

/// A sample group (for presentation purposes only)
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Group<'a> {
    #[serde(
        borrow,
        default,
        rename = "@name",
        skip_serializing_if = "str::is_empty"
    )]
    name: Cow<'a, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<Color>,
}

impl<'a> Group<'a> {
    /// Give a name to the group
    pub fn with_name(self, name: impl Into<Cow<'a, str>>) -> Self {
        Self {
            name: name.into(),
            ..self
        }
    }

    /// Provide a color to associate with the group
    pub fn with_color(self, color: impl Into<Option<Color>>) -> Self {
        Self {
            color: color.into(),
            ..self
        }
    }

    /// Get the name of the group
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the color associated with the group, if any
    pub fn color(&self) -> Option<Color> {
        self.color
    }
}

/// RGB hex value
pub type Color = [u8; 3];

/// Mapping information for a sample file
#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Sample<'a> {
    #[serde(borrow, rename = "@file")]
    file: Cow<'a, std::path::Path>,
    #[serde(rename = "@sample-start", skip_serializing_if = "Option::is_none")]
    sample_start: Option<f64>,
    #[serde(rename = "@sample-stop", skip_serializing_if = "Option::is_none")]
    sample_stop: Option<f64>,
    #[serde(rename = "@gain", skip_serializing_if = "Option::is_none")]
    gain: Option<f64>,
    #[serde(rename = "@group", skip_serializing_if = "Option::is_none")]
    group: Option<isize>,
    #[serde(rename = "@parameter-1", skip_serializing_if = "Option::is_none")]
    parameter_1: Option<f64>,
    #[serde(rename = "@parameter-2", skip_serializing_if = "Option::is_none")]
    parameter_2: Option<f64>,
    #[serde(rename = "@parameter-3", skip_serializing_if = "Option::is_none")]
    parameter_3: Option<f64>,
    #[serde(rename = "@reverse", skip_serializing_if = "Option::is_none")]
    reverse: Option<bool>,
    #[serde(rename = "@zone-logic", skip_serializing_if = "Option::is_none")]
    zone_logic: Option<ZoneLogic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<Key>,
    #[serde(skip_serializing_if = "Option::is_none")]
    velocity: Option<ZoneInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    select: Option<ZoneInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#loop: Option<Loop>,
}

impl<'a> Sample<'a> {
    /// Set the file path of the sample
    pub fn with_file(self, file: impl Into<Cow<'a, std::path::Path>>) -> Self {
        Self {
            file: file.into(),
            ..self
        }
    }

    /// Set the start point for the sample (in frames)
    pub fn with_sample_start(self, sample_start: impl Into<Option<f64>>) -> Self {
        Self {
            sample_start: sample_start.into(),
            ..self
        }
    }

    /// Set the end point for the sample (in frames)
    pub fn with_sample_stop(self, sample_stop: impl Into<Option<f64>>) -> Self {
        Self {
            sample_stop: sample_stop.into(),
            ..self
        }
    }

    /// Set the gain for the sample
    pub fn with_gain(self, gain: impl Into<Option<f64>>) -> Self {
        Self {
            gain: gain.into(),
            ..self
        }
    }

    /// Put the sample in a group
    pub fn with_group(self, group: impl Into<Option<isize>>) -> Self {
        Self {
            group: group.into(),
            ..self
        }
    }

    /// Set the first parameter
    pub fn with_parameter_1(self, parameter_1: impl Into<Option<f64>>) -> Self {
        Self {
            parameter_1: parameter_1.into(),
            ..self
        }
    }

    /// Set the second parameter
    pub fn with_parameter_2(self, parameter_2: impl Into<Option<f64>>) -> Self {
        Self {
            parameter_2: parameter_2.into(),
            ..self
        }
    }

    /// Set the third parameter
    pub fn with_parameter_3(self, parameter_3: impl Into<Option<f64>>) -> Self {
        Self {
            parameter_3: parameter_3.into(),
            ..self
        }
    }

    /// Set whether the sample should be played in reverse
    pub fn with_reverse(self, reverse: impl Into<Option<bool>>) -> Self {
        Self {
            reverse: reverse.into(),
            ..self
        }
    }

    /// Choose an algorithm for sample selection when zones overlap
    pub fn with_zone_logic(self, zone_logic: impl Into<Option<ZoneLogic>>) -> Self {
        Self {
            zone_logic: zone_logic.into(),
            ..self
        }
    }

    /// Set the key range for the sample
    pub fn with_key(self, key: impl Into<Option<Key>>) -> Self {
        Self {
            key: key.into(),
            ..self
        }
    }

    /// Set the velocity range for the sample
    pub fn with_velocity(self, velocity: impl Into<Option<ZoneInfo>>) -> Self {
        Self {
            velocity: velocity.into(),
            ..self
        }
    }

    /// Set the "select" range for the sample
    pub fn with_select(self, select: impl Into<Option<ZoneInfo>>) -> Self {
        Self {
            select: select.into(),
            ..self
        }
    }

    /// Set the loop behavior of the sample
    pub fn with_loop(self, r#loop: impl Into<Option<Loop>>) -> Self {
        Self {
            r#loop: r#loop.into(),
            ..self
        }
    }

    /// Get the path to the sample file
    pub fn file(&self) -> &std::path::Path {
        &self.file
    }

    /// Get the sample's start point (in frames)
    pub fn sample_start(&self) -> Option<f64> {
        self.sample_start
    }

    /// Get the sample's end point (in frames)
    pub fn sample_stop(&self) -> Option<f64> {
        self.sample_stop
    }

    /// Get the sample's gain
    pub fn gain(&self) -> Option<f64> {
        self.gain
    }

    /// Get the group associated with the sample, if any
    pub fn group(&self) -> Option<isize> {
        self.group
    }

    /// Get the value of the first parameter
    pub fn parameter_1(&self) -> Option<f64> {
        self.parameter_1
    }

    /// Get the value of the second parameter
    pub fn parameter_2(&self) -> Option<f64> {
        self.parameter_2
    }

    /// Get the value of the third parameter
    pub fn parameter_3(&self) -> Option<f64> {
        self.parameter_3
    }

    /// Get the playback reversal for the sample
    pub fn reverse(&self) -> Option<bool> {
        self.reverse
    }

    /// Get the overlap behavior for the sample
    pub fn zone_logic(&self) -> Option<ZoneLogic> {
        self.zone_logic
    }

    /// Get the sample's key range
    pub fn key(&self) -> &Option<Key> {
        &self.key
    }

    /// Get the sample's velocity range
    pub fn velocity(&self) -> &Option<ZoneInfo> {
        &self.velocity
    }

    /// Get the sample's "select" range
    pub fn select(&self) -> &Option<ZoneInfo> {
        &self.select
    }

    /// Get the sample's loop behavior
    pub fn r#loop(&self) -> &Option<Loop> {
        &self.r#loop
    }
}

/// Specify behavior when multiple samples occupy the same zone
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ZoneLogic {
    /// Play this sample regardless of zone overlap
    AlwaysPlay,
    /// Alternate this sample with others in the overlapping region
    RoundRobin,
}

/// Mapping data relating to notes played
#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Key {
    #[serde(rename = "@root", default, skip_serializing_if = "Option::is_none")]
    root: Option<u8>,
    #[serde(rename = "@track", default, skip_serializing_if = "Option::is_none")]
    track: Option<f64>,
    #[serde(rename = "@tune", default, skip_serializing_if = "Option::is_none")]
    tune: Option<f64>,
    #[serde(rename = "@low", default, skip_serializing_if = "Option::is_none")]
    low: Option<u8>,
    #[serde(rename = "@high", default, skip_serializing_if = "Option::is_none")]
    high: Option<u8>,
    #[serde(rename = "@low-fade", default, skip_serializing_if = "Option::is_none")]
    low_fade: Option<u8>,
    #[serde(
        rename = "@high-fade",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    high_fade: Option<u8>,
}

impl Key {
    /// Set the root pitch of the sample
    pub fn with_root(self, root: impl Into<Option<u8>>) -> Self {
        Self {
            root: root.into(),
            ..self
        }
    }

    /// Set the keytrack amount (0 to 2)
    pub fn with_track(self, track: impl Into<Option<f64>>) -> Self {
        Self {
            track: track.into(),
            ..self
        }
    }

    /// Set the fine tuning for the sample
    pub fn with_tune(self, tune: impl Into<Option<f64>>) -> Self {
        Self {
            tune: tune.into(),
            ..self
        }
    }

    /// Set the lower end of the pitch range
    pub fn with_low(self, low: impl Into<Option<u8>>) -> Self {
        Self {
            low: low.into(),
            ..self
        }
    }

    /// Set the upper end of the pitch range
    pub fn with_high(self, high: impl Into<Option<u8>>) -> Self {
        Self {
            high: high.into(),
            ..self
        }
    }

    /// Set the length of the lower fade region
    pub fn with_low_fade(self, low_fade: impl Into<Option<u8>>) -> Self {
        Self {
            low_fade: low_fade.into(),
            ..self
        }
    }

    /// Set the length of the upper fade region
    pub fn with_high_fade(self, high_fade: impl Into<Option<u8>>) -> Self {
        Self {
            high_fade: high_fade.into(),
            ..self
        }
    }

    /// Get the sample's root pitch
    pub fn root(&self) -> Option<u8> {
        self.root
    }

    /// Get the sample's keytrack amount
    pub fn track(&self) -> Option<f64> {
        self.track
    }

    /// Get the sample's fine tuning
    pub fn tune(&self) -> Option<f64> {
        self.tune
    }

    /// Get the lower end of the pitch range
    pub fn low(&self) -> Option<u8> {
        self.low
    }

    /// Get the upper end of the pitch range
    pub fn high(&self) -> Option<u8> {
        self.high
    }

    /// Get the length of the lower fade region
    pub fn low_fade(&self) -> Option<u8> {
        self.low_fade
    }

    /// Get the length of the upper fade region
    pub fn high_fade(&self) -> Option<u8> {
        self.high_fade
    }
}

/// Generic mapping with endpoints and fade distances
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ZoneInfo {
    #[serde(rename = "@low", default, skip_serializing_if = "Option::is_none")]
    low: Option<u8>,
    #[serde(rename = "@high", default, skip_serializing_if = "Option::is_none")]
    high: Option<u8>,
    #[serde(rename = "@low-fade", default, skip_serializing_if = "Option::is_none")]
    low_fade: Option<u8>,
    #[serde(
        rename = "@high-fade",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    high_fade: Option<u8>,
}

impl ZoneInfo {
    /// Set the lower end of the region
    pub fn with_low(self, low: impl Into<Option<u8>>) -> Self {
        Self {
            low: low.into(),
            ..self
        }
    }

    /// Set the upper end of the region
    pub fn with_high(self, high: impl Into<Option<u8>>) -> Self {
        Self {
            high: high.into(),
            ..self
        }
    }

    /// Set the length of the lower fade region
    pub fn with_low_fade(self, low_fade: impl Into<Option<u8>>) -> Self {
        Self {
            low_fade: low_fade.into(),
            ..self
        }
    }

    /// Set the length of the upper fade region
    pub fn with_high_fade(self, high_fade: impl Into<Option<u8>>) -> Self {
        Self {
            high_fade: high_fade.into(),
            ..self
        }
    }

    /// Get the lower end of the region
    pub fn low(&self) -> Option<u8> {
        self.low
    }

    /// Get the upper end of the region
    pub fn high(&self) -> Option<u8> {
        self.high
    }

    /// Get the length of the lower fade region
    pub fn low_fade(&self) -> Option<u8> {
        self.low_fade
    }

    /// Get the length of the upper fade region
    pub fn high_fade(&self) -> Option<u8> {
        self.high_fade
    }
}

/// Looping behavior for a sample
#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Loop {
    #[serde(rename = "@mode", skip_serializing_if = "Option::is_none")]
    mode: Option<LoopMode>,
    #[serde(rename = "@start", skip_serializing_if = "Option::is_none")]
    start: Option<f64>,
    #[serde(rename = "@stop", skip_serializing_if = "Option::is_none")]
    stop: Option<f64>,
    #[serde(rename = "@fade", skip_serializing_if = "Option::is_none")]
    fade: Option<f64>,
}

impl Loop {
    /// Set the sample's loop mode
    pub fn with_mode(self, mode: impl Into<Option<LoopMode>>) -> Self {
        Self {
            mode: mode.into(),
            ..self
        }
    }

    /// Set the start point of the loop
    pub fn with_start(self, start: impl Into<Option<f64>>) -> Self {
        Self {
            start: start.into(),
            ..self
        }
    }

    /// Set the end point of the loop
    pub fn with_stop(self, stop: impl Into<Option<f64>>) -> Self {
        Self {
            stop: stop.into(),
            ..self
        }
    }

    /// Set the amount of crossfade when crossing the loop point
    pub fn with_fade(self, fade: impl Into<Option<f64>>) -> Self {
        Self {
            fade: fade.into(),
            ..self
        }
    }

    /// Get the loop mode
    pub fn mode(&self) -> Option<LoopMode> {
        self.mode
    }

    /// Get the start point of the loop
    pub fn start(&self) -> Option<f64> {
        self.start
    }

    /// Get the end point of the loop
    pub fn stop(&self) -> Option<f64> {
        self.stop
    }

    /// Get the crossfade amount
    pub fn fade(&self) -> Option<f64> {
        self.fade
    }
}

/// Traversal mode
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LoopMode {
    /// Do not loop
    #[default]
    Off,
    /// Loop in the playback direction
    Loop,
    /// Loop in alternating directions
    PingPong,
}
