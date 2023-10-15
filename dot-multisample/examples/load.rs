fn main() {
    let Some(path) = std::env::args_os().nth(1) else {
        eprintln!("Usage: load [path_to_multisample]");
        std::process::exit(1);
    };

    let mut path = std::path::PathBuf::from(path);

    // in case we need to extract a zipped version
    let tmp_dir = tempfile::tempdir().expect("Could not create temporary directory");

    // multisample can either be a directory or a ZIP archive
    if path.is_file() {
        // read the ZIP archive
        let zip_file = std::fs::File::open(&path).expect("Failed to open archive file");
        let reader = std::io::BufReader::new(zip_file);
        let mut archive = zip::ZipArchive::new(reader).expect("Could not interpret archive as ZIP");

        // extract it into the temp directory
        path = tmp_dir.path().to_owned();
        archive
            .extract(&path)
            .expect("Failed to extract ZIP archive contents");
    }

    // read manifest file
    let content = std::fs::read_to_string(path.join("multisample.xml"))
        .expect("Could not read manifest file");

    // parse contents of manifest file into our format
    let config: dot_multisample::Multisample =
        quick_xml::de::from_str(&content).expect("Could not parse file as multisample");

    // everything below this point is just printing the contents of the multisample

    println!("       Name\t{}", config.name());
    println!("  Generator\t{}", config.generator());
    println!("   Category\t{}", config.category());
    println!("    Creator\t{}", config.creator());
    println!("Description\t{}", config.description());

    print!("   Keywords\t");
    let keywords = config.keywords();
    if keywords.is_empty() {
        println!("None")
    } else {
        println!("{}", keywords.join(", "));
    }

    print!("     Groups\t");
    let groups = config.groups();
    if groups.is_empty() {
        println!("None")
    } else {
        println!();
        for (idx, group) in groups.iter().enumerate() {
            println!("\t[{idx}] {} ({:?})", group.name(), group.color());
        }
    }

    print!("    Samples\t");
    let samples = config.samples();
    if samples.is_empty() {
        println!("None");
    } else {
        println!();
        for (idx, sample) in samples.iter().enumerate() {
            let name = sample.file();
            println!("\t[{idx}] {name:?}");
            if let Ok(meta) = path.join(name).metadata() {
                println!("\t    exists (size {} bytes)", meta.len());
            } else {
                println!("does not exist");
            }
            if let Some(g) = sample.group() {
                println!("\t    group {g}");
            } else {
                println!("\t    ungrouped");
            }
            if let Some(key) = sample.key() {
                if let (Some(low), Some(high)) = (key.low(), key.high()) {
                    println!("\t    notes {low} to {high}");
                }
            }
        }
    }
}
