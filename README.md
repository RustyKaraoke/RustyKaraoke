# RustyKaraoke

RustyKaraoke is a karaoke player written in Rust initially made for Nick Karaoke (NCN/MIDI+CUR) libraries. It is also planned to support ASS and SRT subtitles, CDG and MP3+G files, and full music videos with subtitles.

## Planned Features

- [ ] NCN File support + MIDI+CUR
- [ ] CDG File support
- [ ] Lyrics display
- [ ] Subtitles support
- [ ] Video player
- [ ] Custom themes (colors, fonts, etc.)


### Planned (extra) features

- [ ] VST/VSTi plugin support
- [ ] Audio effects
- [ ] Lyrics sync editor (CUR editor)

## Why Rust?

The main reason for using Rust is actually because the author only knew Rust at the time of creating this. It is also an opportunity to learn about GUI and media development in Rust.

## Why not use existing software?

Nick Karaoke is closed source and is essentially abandonware. There are cracked repacks out there in the Thai bootleg scene, but they are not updated and are not very stable. There are also patches of it such as the Karalight softsynth, which is not really stable either.

There is also an open source NCN player called [HandyKaraoke](https://github.com/pie62/HandyKaraoke), but its development seems to be dead since 2019. It is also written using C++ and Qt, which is not really my cup of tea.

## Planned tech stack

- egui for the GUI
- RustAudio stack for audio (vst-rs, cpal, rodio, etc.)
- ffmpeg for video decoding
- [oxisynth](https://crates.io/crates/oxisynth) for MIDI playback


## Extra notes

Developers are welcome and needed for help with this project.
Audio engineering knowledge is needed for implementing the audio effects and VST plugins.


## List of help needed

- Audio playback optimization
- GUI design and implementation with egui
- MIDI playback controls and seeking
- Video playback
- Subtitle support