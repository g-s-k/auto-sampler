# auto-sampler

A tool for automatically sampling sound from a MIDI device.

## Structure

- A crate that defines the data structures and logic for the note traversal process is located in [autosam](./autosam).
- A crate that defines a command-line application is located in [multirec](./multirec). It contains all the I/O, error
  handling, and glue necessary to interact with audio and MIDI devices and save the sample files.
