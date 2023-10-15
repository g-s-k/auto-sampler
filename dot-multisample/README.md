# dot-multisample

Bindings for the [.multisample](https://github.com/bitwig/multisample) manifest file format

## Usage

The provided bindings have been created for use with [quick-xml](https://crates.io/crates/quick-xml)
as (at time of writing) it has the best Serde support for XML documents.

See [the "load" example](https://github.com/g-s-k/auto-sampler/tree/main/dot-multisample/examples/load.rs)
for a more practical demonstration, but here is a short snippet:

```rust
let path: &std::path::Path = "path/to/Instrument.multisample".as_ref();
let manifest = std::fs::read_to_string(path.join("multisample.xml")).unwrap();

let config: Multisample = quick_xml::de::from_str(&manifest).unwrap();

println!("{} by {}", config.name(), config.creator());
```