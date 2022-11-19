use encoding::{
    all::{UTF_8, WINDOWS_874},
    DecoderTrap, Encoding,
};
use oxisynth::{Settings, SoundFont, Synth, SynthDescriptor};
/// MIDI player code
use std::{
    fmt,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};

use midly::{
    live::SystemRealtime,
    num::{u15, u7},
    Format, Smf,
};
use nodi::{timers::Ticker, Connection, Event, MidiEvent, Moment, Player, Sheet, Timer};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, BuildStreamError, DefaultStreamConfigError, OutputCallbackInfo, PlayStreamError,
    SampleFormat, Stream,
};
use log::{debug, error, info, trace, warn};
use parking_lot::Mutex;

use crate::tick::{cur_test, scroll, CurData};
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
    synth: Arc<Mutex<Synth>>,
    _stream: Stream,
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
        })
    }
}

impl Connection for Fluid {
    fn play(&mut self, msg: MidiEvent) -> bool {
        use nodi::midly::MidiMessage as M;

        let mut fl = self.synth.lock();
        let c = msg.channel.as_int() as u32;
        let res = match msg.message {
            M::NoteOff { key, .. } => {
                trace!("note off: {} {}", c, key);

                fl.send_event(oxisynth::MidiEvent::NoteOff {
                    channel: c as u8,
                    key: u8::from(key),
                })
            }
            M::NoteOn { key, vel } => {
                trace!("note on: {} {} {}", c, key, vel);
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
        // if msg == SystemRealtime::Reset {
        // 	if let Err(e) = self.synth.lock().unwrap().program_reset() {
        // 		log::error!(target: "midi_event", "failed to reset: {e}");
        // 	}
        // }
        self.all_notes_off();
        self.synth.lock().program_reset();
    }
}

pub async fn run() {
    let data = fs::read("44706.MID").unwrap();
    let smf = Smf::parse(&data).unwrap();
    // debug!("smf: {:?}", smf.header.timing);

    let (timer, res) = {
        // let timer =
        let timer = Ticker::try_from(smf.header.timing).unwrap();
        let res = match smf.header.timing {
            midly::Timing::Metrical(i) => u16::from(i),
            midly::Timing::Timecode(fps, i) => fps.as_int() as u16 * i as u16, //FIXME
        };
        (timer, res)
    };

    // turn header.timing into SMPTE
    // let tempo = timer.clone().sleep_duration(1);

    // debug!("tempo: {:?}", tempo);
    // get tempo from smf
    // let t = timer.sleep_duration(1);

    let sheet = match smf.header.format {
        Format::SingleTrack | Format::Sequential => Sheet::sequential(&smf.tracks),
        Format::Parallel => Sheet::parallel(&smf.tracks),
    };

    println!("format: {:?}", sheet.len() as u16);

    let p = Fluid::new(DEFAULT_SOUNDFONT).unwrap();
    // let a = p.synth.lock().get_pitch_bend(0).unwrap();

    // println!("pitch bend: {}", a);

    let mut player = MidPlayer::new(timer, p, res);

    // get midi ticks per second
    let res = sheet.to_vec();
    debug!("res: {:?}", res.len());
    player.play(&sheet);

    // get midi resolution of sheet

    // println!("{:#?}", sheet);
    // println!("{:?}", timer);
}
// this player is very mid
pub struct MidPlayer<T: Timer, C: Connection> {
    pub con: C,
    pub res: u16,
    timer: T,
}

impl<T: Timer, C: Connection> MidPlayer<T, C> {
    pub fn new(timer: T, con: C, res: u16) -> Self {
        Self { con, timer, res }
    }

    pub fn set_timer(&mut self, timer: T) -> T {
        std::mem::replace(&mut self.timer, timer)
    }

    pub fn play(&mut self, sheet: &[Moment]) -> bool {
        let mut counter = 0_u32;

        // read file
        // let file = std::fs::read("30664.CUR").unwrap();
        let mut file = File::open("44706.CUR").unwrap();

        let mut buf = vec![];
        // read all bytes
        file.read_to_end(&mut buf).unwrap();
        // parse file
        let cur = CurData::read(buf);
        let mut t = cur
            .into_tick()
            .iter()
            .map(|x| *x as u32)
            .collect::<Vec<_>>();

        // read the lyrics file, excluding the first 4 lines
        let mut lyrics_file = File::open("44706.LYR").unwrap();

        let mut buf = Vec::new();
        lyrics_file.read_to_end(&mut buf).unwrap();

        let (text, _enc) = encoding::decode(&buf, DecoderTrap::Ignore, WINDOWS_874);

        let lyrics = match text {
            Ok(txt) => txt,
            Err(e) => {
                println!("Error: {:?}", e);
                return false;
            }
        };

        // get the first line of the lyrics file
        let title = lyrics.lines().next().unwrap();

        println!("Playing: {}", title);
        let author = lyrics.lines().nth(1).unwrap();
        println!("Author: {}", author);

        let key = lyrics.lines().nth(2).unwrap();
        println!("Key: {}", key);

        // let lyrics = WINDOWS_874.decode(&buf, DecoderTrap::Strict).unwrap();

        let lyrics = lyrics.lines().skip(4).collect::<Vec<&str>>().join("\n").chars().collect::<Vec<char>>();

        // let lyrics = lyrics.chars().collect::<Vec<char>>();

        // println!("{:?}", lyrics);

        let mut bpm: u32 = 0;

        debug!("{} characters to be scrolled in lyrics file", lyrics.len());

        // get smpte time
        // let funny: u15 = self.timer.into();

        // let mut time_cache = 0_u32;

        // index cursor for each lyrics character
        let mut lyric_index = 0_u32;

        for (i, moment) in sheet.iter().enumerate() {
            // cur_test(i as u16);

            // there are `res` ticks per quarter note

            // debug!("moment: {:?}", i);

            // debug!("res: {}", self.res as usize);
            let time: f32 = (i as f32) / self.res as f32;
            // debug!("a: {}", a);

            let mid_time = time / bpm as f32 * 60.0;
            let cur_time = (mid_time * bpm as f32 * 24.0 / 60.0) as u32;

            // println!("cur_time: {}", cur_time as u16);
            // debug!("mid_time: {}", mid_time);
            let time_display = cur_time;
            if t.contains(&time_display) {

                // we run this twice because encoding's bit weird
                for _ in 0..2 {
                    if let Some(c) = lyrics.get(lyric_index as usize) {
                        // print the character
                        scroll(*c);

                        // increment the index
                        lyric_index += 1;
                    }
                }

                // remove tick from t
                let index = t.iter().position(|&r| r == time_display).unwrap();
                t.remove(index);

                // if time_display != time_cache {}
                // println!("{} is in the file", cur_time as u16);
                // time_cache = time_display;
                // then we cache it so we don't print it again
            } /* else if t.iter().all(|f| i > *f as usize) {
                  // for the skipped times, we scroll all of them at once
                  // remove all the lesser ticks from t and scroll them

                  let current = t.clone();
                  let to_scroll = current
                      .iter()
                      .filter(|f| i > **f as usize)
                      .collect::<Vec<&u16>>();
                  info!("there are {} ticks that was skipped", to_scroll.len());
                  info!("skipped ticks: {:?}", to_scroll);

                  for _ in 0..to_scroll.len() {
                      let index = t.iter().position(|&r| r == *to_scroll[0]);

                      if let Some(index) = index {
                          scroll(&lyrics.remove(0).to_string());
                          t.remove(index);
                      }
                  }
              } */

            // else the current time is bigger than any of the times in the file

            if !moment.is_empty() {
                self.timer.sleep(counter);
                // info!("playing moment {}", cur_time);
                counter = 0;

                // get play progress

                // get moment index
                // for (i, event) in moment.iter().enumerate() { }
                for event in &moment.events {
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
                            if !self.con.play(*msg) {
                                return false;
                            }
                        }
                        _ => (),
                    };
                }
            }

            // info!("counter: {}", counter);
            counter += 1;
        }

        true
    }
}

#[tokio::test]
async fn test_aa() {
    run().await;
}
