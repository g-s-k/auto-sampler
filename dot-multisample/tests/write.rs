use std::path::Path;

use dot_multisample::*;

fn write<T: serde::Serialize>(data: T) -> String {
    let mut out = String::new();
    let mut s = quick_xml::se::Serializer::new(&mut out);
    s.expand_empty_elements(true);
    s.indent(' ', 2);
    data.serialize(s).unwrap();
    out
}

#[test]
fn empty() {
    assert_eq!(
        write(Multisample::default()),
        include_str!("data/empty.xml")
    );
}

#[test]
fn just_groups() {
    assert_eq!(
        write(Multisample::default().with_groups(vec![
            Group::default().with_name("First"),
            Group::default().with_name("Second"),
        ],)),
        include_str!("data/groups.xml")
    );
}

#[test]
fn more_detailed() {
    assert_eq!(
        write(
            Multisample::default()
                .with_name("1980s FM Synth")
                .with_generator("multirec")
                .with_category("Pad")
                .with_description("Very large plastic synthesizer playing a pad sound")
                .with_keywords(["Pad", "Synth", "Glassy", "Retro"])
                .with_samples([
                    Sample::default().with_file(AsRef::<Path>::as_ref("C2.wav")),
                    Sample::default().with_file(AsRef::<Path>::as_ref("F2.wav")),
                    Sample::default().with_file(AsRef::<Path>::as_ref("A#2.wav")),
                    Sample::default().with_file(AsRef::<Path>::as_ref("D#3.wav")),
                ])
        ),
        include_str!("data/details.xml")
    );
}
