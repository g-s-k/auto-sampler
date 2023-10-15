use std::path::Path;

use dot_multisample::*;

#[test]
fn empty() {
    let multi: Multisample = quick_xml::de::from_str(include_str!("data/empty.xml")).unwrap();

    assert_eq!(multi.name(), "");
    assert_eq!(multi.generator(), "");
    assert_eq!(multi.category(), "");
    assert_eq!(multi.creator(), "");
    assert_eq!(multi.description(), "");
    assert_eq!(multi.keywords(), &[] as &[std::borrow::Cow<'_, str>]);
    assert_eq!(multi.groups(), []);
    assert_eq!(multi.samples(), []);
}

#[test]
fn with_xml_header() {
    let multi: Multisample = quick_xml::de::from_str(include_str!("data/with_header.xml")).unwrap();

    assert_eq!(multi.name(), "Test");
}

#[test]
fn just_groups() {
    let multi: Multisample = quick_xml::de::from_str(include_str!("data/groups.xml")).unwrap();

    assert_eq!(
        multi.groups(),
        [
            Group::default().with_name("First"),
            Group::default().with_name("Second")
        ]
    );
}

#[test]
fn more_detailed() {
    let multi: Multisample = quick_xml::de::from_str(include_str!("data/details.xml")).unwrap();

    assert_eq!(multi.name(), "1980s FM Synth");
    assert_eq!(multi.generator(), "multirec");
    assert_eq!(multi.category(), "Pad");
    assert_eq!(multi.creator(), "");
    assert_eq!(
        multi.description(),
        "Very large plastic synthesizer playing a pad sound"
    );
    assert_eq!(multi.keywords(), ["Pad", "Synth", "Glassy", "Retro"]);
    assert_eq!(multi.groups(), []);
    assert_eq!(
        multi.samples(),
        [
            Sample::default().with_file(AsRef::<Path>::as_ref("C2.wav")),
            Sample::default().with_file(AsRef::<Path>::as_ref("F2.wav")),
            Sample::default().with_file(AsRef::<Path>::as_ref("A#2.wav")),
            Sample::default().with_file(AsRef::<Path>::as_ref("D#3.wav")),
        ]
    );
}
