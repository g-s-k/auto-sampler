# multirec

A command-line application built around the [autosam](https://crates.io/crates/autosam) library

## Usage examples

```
$ multirec show audio-hosts

ID      Name
0       CoreAudio
```

```shell
$ multirec show audio-devices

ID      In      Out     Fs Min  Fs Max  Name
2       1       0        44100   96000  MacBook Pro Microphone
3       0       2        44100   96000  MacBook Pro Speakers
```

```
$ multirec test --dry-run

Sample Offset           Event   Pitch   Velo    MIDI
--------------------    -----   -----   ----    ----
                   0    On      C3       127    [144, 48, 127]
               96000    Off     C3       127    [128, 48, 127]
```
