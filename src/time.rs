//! Timing module for playback progress.

use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, AtomicUsize},
        Arc,
    },
};

use chrono::Duration;
use crossbeam::channel::{Receiver, Sender};
use derivative::Derivative;
use hhmmss::Hhmmss;
use log::debug;
use parking_lot::Mutex;
use tokio::task::JoinHandle;

use crate::midi::{self, Fluid, MidiContext, MidiControl, MidiMessage};
// should i make this a singleton?
// or should i make it a struct that is passed around?

#[derive(Derivative)]
#[derivative(Debug, Clone)]
pub struct PlaybackContext {
    pub position: Option<Duration>,
    pub total: Option<Duration>,
    pub backend: Option<PlaybackBackend>,
    // pub player: Option<JoinHandle<()>>,
    pub paused: bool,
}
#[derive(Derivative)]
#[derivative(Debug, Clone)]
pub enum PlaybackBackend {
    Midi { ctx: Arc<Mutex<MidiContext>> },
    Audio,
}

impl PlaybackBackend {
    pub fn get_time(&self) -> Option<String> {
        match self {
            PlaybackBackend::Midi { ctx } => {
                let ctx = ctx.lock();
                let total = ctx.total.unwrap_or_else(Duration::zero);
                let elapsed = ctx.elapsed.unwrap_or_else(Duration::zero);

                let text = format!("{} / {}", elapsed.hhmmss(), total.hhmmss());
                Some(text)
            }
            PlaybackBackend::Audio => None,
        }
    }

    pub fn get_position(&self) -> (usize, usize) {
        match self {
            PlaybackBackend::Midi { ctx } => {
                let ctx = ctx.lock();
                let total = ctx.midi_tick_max.unwrap_or(0);
                let elapsed = ctx.midi_tick.unwrap_or(0);

                (elapsed, total)
            }
            PlaybackBackend::Audio => (0, 0),
        }
    }

    pub fn stop(&mut self) {
        match self {
            PlaybackBackend::Midi { ctx } => {
                let mut ctx = ctx.lock();
                ctx.stop();
            }
            PlaybackBackend::Audio => {}
        }
    }

    pub fn get_backend(&self) -> Arc<Mutex<MidiContext>> {
        match self {
            PlaybackBackend::Midi { ctx } => ctx.clone(),
            PlaybackBackend::Audio => unimplemented!(),
        }
    }
}

impl PlaybackContext {
    pub fn new() -> Self {
        Self {
            position: None,
            total: None,
            backend: None,
            // player: None,
            paused: false,
        }
    }
}

#[derive(Debug)]
pub enum PlaybackEvent {
    Backend(PlaybackBackend),
    Position(usize, Option<f32>, Option<usize>),
    Total(Duration),
    Pause,
    // Load(PathBuf),
    Play(PathBuf),
    Stop,
    Exit,
}
use lazy_static::lazy_static;

// lazy_static! {
//     pub static ref MIDI_PLAYER: Arc<Mutex<MidiControl>> = Arc::new(Mutex::new(MidiControl::new()));
// }
pub fn msg_send(
    midi: Sender<MidiMessage>,
) -> (
    Sender<PlaybackEvent>,
    Arc<Mutex<PlaybackContext>>,
    Sender<()>,
) {
    // crossbeam channel for sending messages to the playback thread
    let (tx, rx) = crossbeam::channel::bounded(0);

    let context = PlaybackContext::new();

    let arc = Arc::new(Mutex::new(context));

    let tx2 = tx.clone();
    let tx3 = tx.clone();

    let mut closing = false;

    let (backtx, backrx) = msg_reciever();

    let arc2 = Arc::clone(&arc);
    let arc3 = Arc::clone(&arc);

    // let (synth, midicon) = midi::init_midi();

    let (mptx, mprx) = crossbeam::channel::unbounded();

    // backtx.clone().send(context.clone()).unwrap();
    let mptx2 = mptx.clone();
    let midi = midi.clone();
    // recieve messages from the playback thread
    tokio::spawn(async move {
        // this is some very scuffed code
        loop {
            if closing {
                break;
            } else {
                // let midipause = midipause.clone();
                if let Ok(msg) = rx.recv() {
                    match msg {
                        PlaybackEvent::Backend(backend) => {
                            println!("backend: {:?}", backend);
                        }
                        PlaybackEvent::Position(position, dur, tick) => {
                            let mut l = arc2.lock();

                            if let Some(time) = dur {
                                // try to convert to chrono duration
                                // let dur = std::time::Duration::from_secs_f32(time);
                                // let dur = Duration::from_std(dur).unwrap();
                            }

                            // println!("position: {:?}", position);
                        }
                        PlaybackEvent::Total(total) => {
                            println!("total: {:?}", total);
                        }
                        PlaybackEvent::Pause => {
                            println!("pause");
                            // backend
                            //     .lock()
                            //     .backend
                            //     .as_ref()
                            //     .unwrap()
                            //     .get_backend()
                            //     .lock()
                            //     .pause();
                            // midipause.0.send(()).unwrap();
                        }
                        PlaybackEvent::Play(file) => {
                            // very hacky var moving code. Thanks rust
                            println!("play");
                            let mprx = mprx.clone();
                            let midi = midi.clone();
                            let tx3 = tx3.clone();
                            let arc3 = arc3.clone();
                            std::thread::spawn(move || {
                                let mut mid =
                                    MidiControl::new(midi.clone(), tx3.clone(), arc3.clone(), mprx);
                                if let Some(back) = arc3.lock().backend.as_ref() {
                                    match back {
                                        PlaybackBackend::Midi { ctx } => {
                                            // ctx.lock().playing = true;

                                            // midi_context seems to be causing bugs
                                            // mid.midi_context = ctx.clone();
                                        }
                                        PlaybackBackend::Audio => {}
                                    }
                                }
                                // midi::run(tx, backend).await;
                                mid.play(&file, None);
                            });
                            // midi::run(tx.clone()).await;
                        }
                        PlaybackEvent::Stop => {
                            println!("stop");
                            let mut l = arc3.lock();
                            if let Some(s) = l.backend.as_mut(){
                                s.stop();
                            }
                        }
                        PlaybackEvent::Exit => {
                            println!("exit");
                            closing = true;
                        }
                    }
                }
            }
        }
    });

    (tx, Arc::clone(&arc), mptx)
}

pub fn msg_reciever() -> (Sender<PlaybackContext>, Receiver<PlaybackContext>) {
    crossbeam::channel::bounded(0)
}

pub async fn process_ctrl_msg(
    rx: Receiver<PlaybackEvent>,
    closing: &mut bool,
    tx: Sender<PlaybackEvent>,
    backend_tx: Sender<PlaybackContext>,
    backend: Arc<Mutex<PlaybackContext>>,
    midipause: (Sender<()>, Receiver<()>),
    // midicon: (Arc<Mutex<Fluid>>,Arc<Mutex<MidiContext>>),
) {
    // debug!("Processing message");

    // backend_tx.send(*backend).unwrap();
}

#[derive(Debug, Clone)]
pub struct Context {
    pub playback: PlaybackContext,
}

impl Context {
    pub fn new() -> Self {
        Self {
            playback: PlaybackContext::new(),
        }
    }
}

// derefmut for context

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
