use anyhow::{anyhow, Result};
use chrono::Duration;
use crossbeam::channel::{Receiver, Sender};
use derivative::Derivative;
use encoding::{
    all::{UTF_8, WINDOWS_874},
    DecoderTrap, Encoding,
};
use lazy_static::lazy_static;
use oxisynth::{Settings, SoundFont, Synth, SynthDescriptor};
use rayon::prelude::IntoParallelRefMutIterator;
/// MIDI player code
use std::{
    fmt,
    fs::{self, File},
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicUsize},
};

use midly::{live::SystemRealtime, Format, Smf, Timing};
use nodi::{
    timers::{Ticker, TimeFormatError},
    Connection, Event, MidiEvent, Moment, Sheet, Timer,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BuildStreamError, DefaultStreamConfigError, OutputCallbackInfo, PlayStreamError, SampleFormat,
    Stream,
};
use log::{debug, error, info, trace, warn};
use parking_lot::{Mutex, RwLock};

use crate::{
    tick::{scroll, CurData},
    time::{PlaybackContext, PlaybackEvent},
};
const DEFAULT_SOUNDFONT: &str = {
    if cfg!(windows) {
        r"C:\soundfonts\default.sf2"
    } else {
        "/usr/share/soundfonts/default.sf2"
    }
};

#[derive(Debug)]
pub enum Error {
    Soundfont {
        path: PathBuf,
        error: fluidlite::Error,
    },
    Fluidlite(fluidlite::Error),
    NoOutputDevice,
    DefaultStreamConfig(DefaultStreamConfigError),
    BuildStream(BuildStreamError),
    PlayStream(PlayStreamError),
}

impl std::error::Error for Error {}

impl From<fluidlite::Error> for Error {
    fn from(e: fluidlite::Error) -> Self {
        Self::Fluidlite(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Soundfont { path, error } => write!(
                f,
                "failed loading the soundfont {} ({})",
                path.display(),
                error
            ),
            Self::Fluidlite(e) => e.fmt(f),
            Self::NoOutputDevice => f.write_str("no audio output device detected"),
            Self::DefaultStreamConfig(e) => e.fmt(f),
            Self::BuildStream(e) => e.fmt(f),
            Self::PlayStream(e) => e.fmt(f),
        }
    }
}

pub struct Fluid {
    pub synth: Arc<Mutex<Synth>>,
    // pub context: Arc<Mutex<MidiContext>>,
    _stream: Stream,
}

// MIDI display for debugging and stuff
#[derive(Debug, Clone, Copy, Ord, PartialEq, PartialOrd, Eq)]
pub struct Note {
    pub note: u8,
    pub velocity: u8,
    pub channel: u8,
}
#[derive(Debug, Clone, Ord, PartialEq, PartialOrd, Eq)]
pub struct Track {
    pub notes: Vec<Note>,
}
#[derive(Debug, Clone, Ord, PartialEq, PartialOrd, Eq)]
pub struct MidiDisplay {
    pub tracks: Vec<Track>,
}

impl MidiDisplay {
    pub fn new() -> Self {
        // vector with 16 tracks
        let mut tracks = Vec::with_capacity(16);

        for _ in 0..16 {
            tracks.push(Track { notes: Vec::new() });
        }

        Self { tracks }
    }

    pub fn note_on(&mut self, note: u8, velocity: u8, channel: u8, track: usize) {
        let track = self.tracks.get_mut(track).unwrap();
        track.notes.push(Note {
            note,
            velocity,
            channel,
        });
    }

    pub fn note_off(&mut self, note: u8, channel: u8, track: usize) {
        // debug!("note off: {} {}", note, channel);
        let track = self.tracks.get_mut(track).unwrap();
        let note = track
            .notes
            .iter()
            .position(|n| n.note == note && n.channel == channel)
            .unwrap_or_default();

        // debug!("note off: {}", note);
        track.notes.remove(note);
    }
}

lazy_static! {
    pub static ref TRACKVIEW: Arc<RwLock<MidiDisplay>> = Arc::new(RwLock::new(MidiDisplay::new()));
}

impl Fluid {
    pub fn new<P: AsRef<Path>>(sf: P) -> Result<Self, Error> {
        let mut fl = Synth::default();

        // Load soundfont
        {
            let mut file = File::open(sf.as_ref()).unwrap();
            let font = SoundFont::load(&mut file).unwrap();
            info!("Loading soundfont {}", sf.as_ref().display());
            fl.add_font(font, true);
        }

        // Initialize the audio stream.
        let err_fn = |e| error!("error [audio stream]: {e}");
        let host = cpal::default_host();
        let dev = host.default_output_device().ok_or(Error::NoOutputDevice)?;

        let config = dev
            .default_output_config()
            .map_err(Error::DefaultStreamConfig)?;
        fl.set_sample_rate(config.sample_rate().0 as f32);
        let synth = Arc::new(Mutex::new(fl));
        let fl = Arc::clone(&synth);

        let format = config.sample_format();
        let config = config.config();

        // config.buffer_size = BufferSize::Fixed(2048);

        // println!("config: {:?}", config);

        let stream = match format {
            SampleFormat::F32 => {
                let stream = dev
                    .build_output_stream(
                        &config,
                        move |data: &mut [f32], _: &OutputCallbackInfo| {
                            // debug!("{:?}", data);
                            fl.lock().write(data);
                        },
                        err_fn,
                    )
                    .unwrap();
                stream.play().unwrap();
                stream
            }
            SampleFormat::I16 => {
                let stream = dev
                    .build_output_stream(
                        &config,
                        move |data: &mut [f32], _: &OutputCallbackInfo| {
                            fl.lock().write(data);
                        },
                        err_fn,
                    )
                    .unwrap();
                stream.play().unwrap();
                stream
            }
            SampleFormat::U16 => {
                let stream = dev
                    .build_output_stream(
                        &config,
                        move |data: &mut [f32], _: &OutputCallbackInfo| {
                            fl.lock().write(data);
                        },
                        err_fn,
                    )
                    .unwrap();
                stream.play().unwrap();
                stream
            }
        };

        Ok(Self {
            synth: Arc::clone(&synth),
            _stream: stream,
            // context: ctx,
        })
    }
    pub fn add_soundfont<P: AsRef<Path>>(&mut self, sf: P) -> Result<(), Error> {
        let mut fl = self.synth.lock();
        // fl.reset();
        let mut file = File::open(sf.as_ref()).unwrap();
        let font = SoundFont::load(&mut file).unwrap();
        info!("Loading soundfont {}", sf.as_ref().display());
        fl.add_font(font, true);
        Ok(())
    }
}

impl Connection for Fluid {
    fn play(&mut self, msg: MidiEvent) -> bool {
        use nodi::midly::MidiMessage as M;

        // println!("MIDI: {:?}", msg);

        // if !self.context.lock().playing {
        //     debug!("playback stopped");
        //     return false;
        // }

        // ???????? NOTE OFF IS NOTEON WITH 0 VELOCITY???? WHAT
        let mut fl = self.synth.lock();
        let c = msg.channel.as_int() as u32;
        let res = match msg.message {
            M::NoteOff { key, .. } => {
                trace!("note off: {} {}", c, key);
                // TRACKVIEW
                //     .write()
                //     .note_off(u8::from(key), c as u8, c as usize);
                fl.send_event(oxisynth::MidiEvent::NoteOff {
                    channel: c as u8,
                    key: u8::from(key),
                })
            }
            M::NoteOn { key, vel } => {
                trace!("note on: {} {} {}", c, key, vel);

                // if u8::from(vel) == 0 {
                //     TRACKVIEW
                //         .write()
                //         .note_off(u8::from(key), c as u8, c as usize);
                // } else {
                //     TRACKVIEW
                //         .write()
                //         .note_on(u8::from(key), u8::from(vel), c as u8, c as usize);
                // }
                fl.send_event(oxisynth::MidiEvent::NoteOn {
                    channel: c as u8,
                    key: u8::from(key),
                    vel: u8::from(vel),
                })
            }
            M::Aftertouch { key, vel } => {
                trace!("aftertouch: {} {} {}", c, key, vel);
                fl.send_event(oxisynth::MidiEvent::PolyphonicKeyPressure {
                    channel: c as u8,
                    key: u8::from(key),
                    value: u8::from(vel),
                })
            }
            M::Controller { controller, value } => {
                trace!("controller: {} {} {}", c, controller, value);
                fl.send_event(oxisynth::MidiEvent::ControlChange {
                    channel: c as u8,
                    ctrl: u8::from(controller),
                    value: u8::from(value),
                })
            }
            M::ProgramChange { program } => {
                trace!("program change: {} {}", c, program);
                fl.send_event(oxisynth::MidiEvent::ProgramChange {
                    channel: c as u8,
                    program_id: u8::from(program),
                })
            }
            M::ChannelAftertouch { vel } => {
                trace!("channel aftertouch: {} {}", c, vel);
                fl.send_event(oxisynth::MidiEvent::ChannelPressure {
                    channel: c as u8,
                    value: u8::from(vel),
                })
            }
            M::PitchBend { bend } => {
                let truebend = u16::from(bend.0); //fucky "u16" type. real value is actually 14 bits but oxisynth expects u16
                trace!("pitch bend: {} {}", c, truebend);
                fl.send_event(oxisynth::MidiEvent::PitchBend {
                    channel: c as u8,
                    value: truebend, //????
                })
            }
        };
        if let Err(e) = res {
            log::debug!(target: "midi_event", "{e}");
        }
        true
    }

    fn send_sys_rt(&mut self, msg: SystemRealtime) {
        if msg == SystemRealtime::Reset {
            self.all_notes_off();
            self.synth.lock().program_reset();
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug, Clone, Default)]
pub struct MidiContext {
    pub playing: bool,
    pub midi_tick: usize,
    pub midi_tick_max: usize,
    pub total: Option<Duration>,
    pub elapsed: Option<Duration>,
    pub seek: bool,
}

impl MidiContext {
    pub fn new() -> Self {
        Self::default()
    }

    // go up the tree and get the midi control

    pub fn stop(&mut self) {
        self.playing = false;
        self.midi_tick = 0;
        self.elapsed = Some(Duration::zero());
    }

    pub fn start(&mut self) {
        self.playing = true;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }
}

/// A [Timer] that lets you toggle playback.
///
/// This type works exactly like [Ticker], but it checks for messages
/// on a [Receiver] and toggles playback if there is one.
///
/// Sending a message to [self.pause] will pause the thread until another
/// message is received.
///
/// # Notes
/// Using [Ticker] is recommended over this, mainly because there is the
/// overhead of [Receiver] with this type.
///
/// Calling [sleep](Self::sleep) will panic if the corresponding end of the
/// receiver is poisoned, see the [mpsc](std::sync::mpsc) documentation for
/// more.
#[derive(Debug)]
pub struct ControlTicker {
    pub ticks_per_beat: u16,
    pub micros_per_tick: f64,
    /// Speed modifier, a value of `1.0` is the default and affects nothing.
    ///
    /// Important: Do not set to 0.0, this value is used as a denominator.
    pub speed: f32,
    /// Messages to this channel will toggle playback.
    pub pause: Receiver<()>,
}

impl ControlTicker {
    /// Creates an instance of [Self] with the given ticks-per-beat.
    /// The tempo will be infinitely rapid, meaning no sleeps will happen.
    /// However this is rarely an issue since a tempo change message will set
    /// it, and this usually happens before any non-0 offset event.
    pub fn new(ticks_per_beat: u16, pause: Receiver<()>) -> Self {
        Self {
            ticks_per_beat,
            pause,
            micros_per_tick: 0.0,
            speed: 1.0,
        }
    }

    /// Will create an instance of [Self] with a provided tempo.
    pub fn with_initial_tempo(ticks_per_beat: u16, tempo: u32, pause: Receiver<()>) -> Self {
        let mut s = Self::new(ticks_per_beat, pause);
        s.change_tempo(tempo);
        s
    }

    ///// Casts `self` to a [Ticker].
    // pub fn to_ticker(&self) -> Ticker {
    //     Ticker {
    //         ticks_per_beat: self.ticks_per_beat,
    //         micros_per_tick: self.micros_per_tick,
    //         speed: self.speed,
    //     }
    // }
}

impl Timer for ControlTicker {
    fn change_tempo(&mut self, tempo: u32) {
        let micros_per_tick = tempo as f64 / self.ticks_per_beat as f64;
        self.micros_per_tick = micros_per_tick;
    }

    fn sleep_duration(&mut self, n_ticks: u32) -> std::time::Duration {
        let t = self.micros_per_tick * n_ticks as f64 / self.speed as f64;
        if t > 0.0 {
            std::time::Duration::from_micros(t as u64)
        } else {
            std::time::Duration::default()
        }
    }

    /// Same with [Ticker::sleep], except it checks if there are any messages on
    /// [self.pause], if there is a message, waits for another one before
    /// continuing with the sleep.
    fn sleep(&mut self, n_ticks: u32) {
        //BUG: egui Deadlock when pausing
        // Check if we're supposed to be paused.
        if self.pause.try_recv().is_ok() {
            // Wait for the next message in order to continue, continue.
            // self.pause.recv().unwrap();
            debug!("paused");
            self.pause
                .recv()
                .unwrap_or_else(|e| panic!("Ticker: pause channel receive failed: {:?}", e));
        }

        trace!(target: "rusty_karaoke::midi::ControlTicker","sleeping for {} ticks", n_ticks);
        let t = self.sleep_duration(n_ticks);

        if !t.is_zero() {
            nodi::timers::sleep(t);
        }
    }
}

fn timing_to_ticker(timing: Timing) -> u16 {
    match timing {
        Timing::Metrical(n) => u16::from(n),
        _ => panic!("Timing must be metrical"),
    }
}
#[derive(Derivative)]
#[derivative(Debug, Clone)]
pub struct MidiControl {
    pub midi_channel: Sender<MidiMessage>,
    pub midi_context: Arc<RwLock<MidiContext>>,
    pub midi: Option<Vec<u8>>,
    pub sheet: Option<Sheet>,
    pub midi_sender: Sender<PlaybackEvent>,
    pub playback_context: Arc<RwLock<PlaybackContext>>,
    pub sigrecv: Receiver<()>,
}

impl MidiControl {
    pub fn new(
        midi_channel: Sender<MidiMessage>,
        msg: crossbeam::channel::Sender<crate::time::PlaybackEvent>,
        ctx: Arc<RwLock<crate::time::PlaybackContext>>,
        rx: Receiver<()>,
    ) -> Self {
        let midicon = Arc::new(RwLock::new(MidiContext::new()));
        Self {
            midi_channel,
            midi_context: midicon,
            midi: None,
            sheet: None,
            midi_sender: msg,
            playback_context: ctx,
            sigrecv: rx,
        }
    }

    pub fn set_context(&mut self, ctx: Arc<RwLock<PlaybackContext>>) {
        self.playback_context = ctx;
    }

    //todo: async
    //BUG: the song just perpetually stops if you try to play it again after stopping it
    pub fn play(&mut self, path: &Path, pos: Option<usize>) {
        let tick = self.midi_context.read().midi_tick;
        self.midi_context.write().playing = true;

        let data = fs::read(path).unwrap();

        self.midi = Some(data.clone());

        self.midi = Some(fs::read(path).unwrap());
        let smf = Smf::parse(&data).unwrap();
        let timer = ControlTicker::new(timing_to_ticker(smf.header.timing), self.sigrecv.clone());

        let res = {
            // let timer =
            // nodi::timers::ControlTicker;
            match smf.header.timing {
                midly::Timing::Metrical(i) => u16::from(i),
                midly::Timing::Timecode(fps, i) => fps.as_int() as u16 * i as u16, //FIXME
            }
        };

        let sheet = match smf.header.format {
            Format::SingleTrack | Format::Sequential => Sheet::sequential(&smf.tracks),
            Format::Parallel => Sheet::parallel(&smf.tracks),
        };

        self.sheet = Some(sheet);

        let mut player = MidPlayer::new(
            timer,
            self.midi_channel.clone(),
            res,
            self.midi_sender.clone(),
            self.playback_context.clone(),
            tick,
            self.midi_context.clone(),
        );

        // i am stuck in a prison of my own creation
        if let Some(sheet) = &self.sheet {
            self.midi_context.write().midi_tick_max = sheet.len();
            // we build yet another ticker because of rust borrowing rules
            self.midi_context.write().total = Some(
                Duration::from_std(Ticker::try_from(smf.header.timing).unwrap().duration(sheet))
                    .unwrap(),
            );
            self.midi_context.write().playing = true;
            player.play(sheet);
            // self.midi_context.write().playing = false;
        }
    }
}

// this player is very mid
pub struct MidPlayer {
    pub con: Sender<MidiMessage>,
    pub res: u16,
    pub msg: crossbeam::channel::Sender<crate::time::PlaybackEvent>,
    pub ctx: Arc<RwLock<crate::time::PlaybackContext>>,
    pub pos: usize,
    pub midi_context: Arc<RwLock<MidiContext>>,
    timer: ControlTicker,
    pos_lock: bool,
}

impl MidPlayer {
    pub fn new(
        timer: ControlTicker,
        con: Sender<MidiMessage>,
        res: u16,
        msg: crossbeam::channel::Sender<crate::time::PlaybackEvent>,
        ctx: Arc<RwLock<crate::time::PlaybackContext>>,
        pos: usize,
        midi_context: Arc<RwLock<MidiContext>>,
    ) -> Self {
        Self {
            con,
            timer,
            res,
            msg,
            ctx,
            pos,
            midi_context,
            pos_lock: false,
        }
    }

    pub fn play(&mut self, sheet: &[Moment]) -> bool {
        let mut counter = 0_u32;

        //
        // The lyrics player will now be disabled for a while until I rewrite the player
        //
        //

        // read file
        // let file = std::fs::read("30664.CUR").unwrap();
        // let mut file = File::open("44706.CUR").unwrap();

        // let mut buf = vec![];

        self.ctx.write().backend = Some(crate::time::PlaybackBackend::Midi {
            ctx: self.midi_context.clone(),
        });

        // read all bytes
        // file.read_to_end(&mut buf).unwrap();
        // parse file
        // let cur = CurData::read(buf);
        // let mut t = cur
        //     .into_tick()
        //     .iter()
        //     .map(|x| *x as u32)
        //     .collect::<Vec<_>>();

        // read the lyrics file, excluding the first 4 lines
        // let mut lyrics_file = File::open("44706.LYR").unwrap();

        // let mut buf = Vec::new();
        // lyrics_file.read_to_end(&mut buf).unwrap();

        // let (text, _enc) = encoding::decode(&buf, DecoderTrap::Ignore, WINDOWS_874);

        // let lyrics = match text {
        //     Ok(txt) => txt,
        //     Err(e) => {
        //         println!("Error: {:?}", e);
        //         return false;
        //     }
        // };
        // get the first line of the lyrics file
        // let title = lyrics.lines().next().unwrap();

        // println!("Playing: {}", title);
        // let author = lyrics.lines().nth(1).unwrap();
        // println!("Author: {}", author);

        // let key = lyrics.lines().nth(2).unwrap();
        // println!("Key: {}", key);

        // // let lyrics = WINDOWS_874.decode(&buf, DecoderTrap::Strict).unwrap();

        // let lyrics = lyrics
        //     .lines()
        //     .skip(4)
        //     .collect::<Vec<&str>>()
        //     .join("\n")
        //     .chars()
        //     .collect::<Vec<char>>();

        // let lyrics = lyrics.chars().collect::<Vec<char>>();

        // println!("{:?}", lyrics);

        let mut bpm: u32 = 0;

        // debug!("{} characters to be scrolled in lyrics file", lyrics.len());

        // get smpte time
        // let funny: u15 = self.timer.into();

        // let mut time_cache = 0_u32;

        // index cursor for each lyrics character
        // let mut lyric_index = 0_u32;
        // self.midi_context.write().midi_tick_max = sheet.len();

        // todo: rewrite this player code so users can scroll it

        // while let Some(moment) = sheet.iter().next() {
        // }

        // let tick = self.midi_context.lock().midi_tick.unwrap_or(0);
        fn reverse_cur_time_to_miditick(cur_time: u32, bpm: u32) -> u32 {
            // reverse the formula for cur_time into i (midi tick)
            let mid_time = (cur_time as f32) / bpm as f32 * 60.0;
            // please tell me this works lol
            (mid_time * bpm as f32 * 24.0 / 60.0) as u32
        }

        let real_tick = reverse_cur_time_to_miditick(self.pos as u32, bpm);

        // todo: rewrite this without iterator so we can kind of rewind

        // for (i, moment) in sheet.iter().skip(real_tick as usize).enumerate() {
        //     let i = i + self.pos;
        //     let time: f32 = (i as f32) / self.res as f32;

        //     let mid_time = time / bpm as f32 * 60.0;
        //     let cur_time = (mid_time * bpm as f32 * 24.0 / 60.0) as u32;

        //     // debug!("test reverse: {}", reverse_cur_time_to_miditick(cur_time, bpm));

        //     let time = (mid_time * 1_000_000.0) as u64;
        //     // let mut midicon = self.midi_context.lock();
        //     self.midi_context.write().elapsed = Some(Duration::microseconds(time as i64));
        //     self.midi_context.write().midi_tick = Some(i);

        //     self.pos = cur_time as usize;

        //     if !self.midi_context.read().playing {
        //         return false;
        //     }

        //     self.msg
        //         .send(crate::time::PlaybackEvent::Position(
        //             cur_time as usize,
        //             Some(mid_time),
        //             Some(i),
        //         ))
        //         .unwrap_or_default();

        //     // println!("cur_time: {}", cur_time as u16);
        //     // debug!("mid_time: {}", mid_time);
        //     // let time_display = cur_time;
        //     // if t.contains(&time_display) {
        //     //     // we run this twice because encoding's bit weird
        //     //     for _ in 0..2 {
        //     //         if let Some(c) = lyrics.get(lyric_index as usize) {
        //     //             // print the character
        //     //             scroll(*c);

        //     //             // increment the index
        //     //             lyric_index += 1;
        //     //         }
        //     //     }

        //     //     // remove tick from t
        //     //     let index = t.iter().position(|&r| r == time_display).unwrap();
        //     //     t.remove(index);

        //     //     // if time_display != time_cache {}
        //     //     // println!("{} is in the file", cur_time as u16);
        //     //     // time_cache = time_display;
        //     //     // then we cache it so we don't print it again
        //     // }
        //     /* else if t.iter().all(|f| i > *f as usize) {
        //         // for the skipped times, we scroll all of them at once
        //         // remove all the lesser ticks from t and scroll them

        //         let current = t.clone();
        //         let to_scroll = current
        //             .iter()
        //             .filter(|f| i > **f as usize)
        //             .collect::<Vec<&u16>>();
        //         info!("there are {} ticks that was skipped", to_scroll.len());
        //         info!("skipped ticks: {:?}", to_scroll);

        //         for _ in 0..to_scroll.len() {
        //             let index = t.iter().position(|&r| r == *to_scroll[0]);

        //             if let Some(index) = index {
        //                 scroll(&lyrics.remove(0).to_string());
        //                 t.remove(index);
        //             }
        //         }
        //     } */
        //     // else the current time is bigger than any of the times in the file

        //     if !moment.is_empty() {
        //         self.timer.sleep(counter);
        //         // info!("playing moment {}", cur_time);
        //         counter = 0;

        //         // get play progress

        //         // get moment index
        //         // for (i, event) in moment.iter().enumerate() { }
        //         for event in &moment.events {
        //             // let mut con = self.con.lock();
        //             // get sheet duration
        //             // get event index
        //             match event {
        //                 Event::Tempo(val) => {
        //                     // debug!("tempo: {}", val);
        //                     // bpm is microseconds per beat
        //                     bpm = 60_000 / (val / 1000);
        //                     // println!("bpm: {}", bpm);
        //                     // convert to microseconds per tick
        //                     self.timer.change_tempo(*val)
        //                 }
        //                 Event::Midi(msg) => {
        //                     if self.con.send(MidiMessage::Event(*msg)).is_err() {
        //                         return false;
        //                     }
        //                 }

        //                 _ => {
        //                     debug!("unhandled event: {:?}", event);
        //                 }
        //             };
        //         }
        //     }

        //     // info!("counter: {}", counter);
        //     counter += 1;
        // }

        // rewrite above so you can scroll it

        while self.midi_context.read().playing {
            let time = (self.pos as f32) / self.res as f32;

            let seconds = time / bpm as f32 * 60.0;
            let cur_time = (seconds * bpm as f32 * 24.0 / 60.0) as u32;

            let time = (seconds * 1_000_000.0) as u64;

            if let Some(mut write) = self.midi_context.try_write(){
                write.elapsed = Some(Duration::microseconds(time as i64));

                // if write.seek {
                //     self.pos = write.midi_tick;
                //     write.seek = false;
                // } else {
                //     self.midi_context.write().midi_tick = self.pos;
                // }
            }
            // weird var sync
            // self.pos = self.midi_context.read().midi_tick.unwrap_or_default() as usize;
            // self.midi_context.write().midi_tick = Some(self.pos as usize);
            // self.pos = self.midi_context.read().midi_tick.clone();

            if self.midi_context.read().seek {
                self.pos = self.midi_context.read().midi_tick;
                if let Some(mut write) = self.midi_context.try_write() {
                    write.seek = false;
                }
            } else {
                self.midi_context.write().midi_tick = self.pos;
            }

            // debug!("seek: {}", self.midi_context.try_read().unwrap().seek);

            // debug!("{}", self.pos);


            if let Some(moment) = sheet.get(self.pos as usize) {
                if !moment.is_empty() {
                    self.timer.sleep(counter);
                    // info!("playing moment {}", cur_time);
                    counter = 0;

                    // get play progress

                    // get moment index
                    // for (i, event) in moment.iter().enumerate() { }
                    for event in &moment.events {
                        // let mut con = self.con.lock();
                        // get sheet duration
                        // get event index
                        match event {
                            Event::Tempo(val) => {
                                // debug!("tempo: {}", val);
                                // bpm is microseconds per beat
                                bpm = 60_000 / (val / 1000);
                                // println!("bpm: {}", bpm);
                                // convert to microseconds per tick
                                self.timer.change_tempo(*val)
                            }
                            Event::Midi(msg) => {
                                if self.con.send(MidiMessage::Event(*msg)).is_err() {
                                    return false;
                                }
                            }

                            Event::KeySignature(val, major ) => {
                                // if val is positive, it's a sharp
                                // if val is negative, it's a flat

                                if val.is_negative() {

                                }
                            }

                            _ => {
                                debug!("unhandled event: {:?}", event);
                            }
                        };
                    }
                    // update tick counter to next tick
                }
                counter += 1;
                self.pos += 1;


            }
            // if it's greater than the last tick, stop playing
            if self.pos >= self.midi_context.read().midi_tick_max {
                self.midi_context.write().playing = false;
            }
        }

        true
    }
}


// TODO: Let's make use of mpsc channels and make a dedicated midi thread. Might be a good idea and fixes the Send/Sync issues
// I have a bad habit of rewriting everything and never using it
/// MIDI Messages to send to the MIDI device
pub enum MidiMessage {
    Event(MidiEvent),
    ClearNotes,
    Soundfont(PathBuf),
}

pub enum MidiSynth {
    Oxisynth(Arc<Mutex<Fluid>>),
    External(Arc<Mutex<dyn Connection>>),
}

impl MidiSynth {
    pub fn play(&mut self, msg: MidiEvent) {
        match self {
            MidiSynth::Oxisynth(synth) => {
                let mut synth = synth.try_lock();

                if let Some(synth) = synth.as_mut() {
                    synth.play(msg);
                }
                // synth.play(msg);
            }
            MidiSynth::External(synth) => {
                let mut synth = synth.try_lock();

                if let Some(synth) = synth.as_mut() {
                    synth.play(msg);
                }
            }
        }
    }

    pub fn as_connection(&self) -> Arc<Mutex<dyn Connection>> {
        match self {
            MidiSynth::Oxisynth(synth) => synth.clone(),
            MidiSynth::External(synth) => synth.clone(),
        }
    }

    pub fn set_soundfont(&mut self, path: &Path) -> Result<()> {
        match self {
            MidiSynth::Oxisynth(synth) => {
                synth.lock().add_soundfont(path)?;
            }
            MidiSynth::External(_) => {
                return Err(anyhow!("external midi synth doesn't support soundfonts"));
            }
        }

        Ok(())
    }

    /// Gets the inner FluidSynth instance. Only works if synth is Oxisynth
    pub fn inner_synth(&self) -> Option<&Arc<Mutex<Fluid>>> {
        match self {
            MidiSynth::Oxisynth(synth) => Some(synth),
            MidiSynth::External(_) => None,
        }
    }
}

pub struct MidiDevice {
    pub con: MidiSynth,
    pub msg: Receiver<MidiMessage>,
}

impl MidiDevice {
    pub fn new(rx: Receiver<MidiMessage>, con: Option<MidiSynth>) -> Self {
        let con = con.unwrap_or_else(|| {
            let default_synth = Fluid::new(DEFAULT_SOUNDFONT).unwrap();

            MidiSynth::Oxisynth(Arc::new(Mutex::new(default_synth)))
        });

        Self { con, msg: rx }
    }

    pub fn listen(&mut self) {
        // set logger

        let target = "MIDIThread";

        debug!(
            target: target,
            "MIDI thread started, listening for messages"
        );

        // self.msg.
        while let Ok(msg) = self.msg.recv() {

            match msg {
                MidiMessage::Event(event) => {
                    trace!(target: target, "Got MIDI event: {:?}", event);
                    // let mut con = self.con.as_connection().lock();
                    self.con.play(event);
                }
                MidiMessage::ClearNotes => {
                    trace!(target: target, "Clearing notes");
                    let con = self.con.as_connection();
                    let mut con = con.lock();
                    // con.send_sys_rt(SystemRealtime::Reset);
                    con.all_notes_off();
                }
                MidiMessage::Soundfont(path) => {
                    trace!(target: target, "Setting soundfont to {:?}", path);
                    if self.con.set_soundfont(&path).is_err() {
                        error!(target: target, "Failed to set soundfont");
                    }
                }
                _ => {
                    unimplemented!();
                    // debug!(target: target, "Got unknown MIDI message: {:?}", msg);
                }
            }
        }
    }
}

pub fn midi_thread(rx: Receiver<MidiMessage>, con: Option<MidiSynth>) {
    let mut midi = MidiDevice::new(rx, con);
    midi.listen();
}
