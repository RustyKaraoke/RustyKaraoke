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
use log::{debug, warn};
use parking_lot::{Mutex, RwLock};

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
    Midi { ctx: Arc<RwLock<MidiContext>> },
    Audio,
}

impl PlaybackBackend {
    pub fn get_time(&self) -> Option<String> {
        match self {
            PlaybackBackend::Midi { ctx } => {
                let ctx = ctx.read();
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
                let ctx = ctx.read();
                let total = ctx.midi_tick_max;
                let elapsed = ctx.midi_tick;

                (elapsed, total)
            }
            PlaybackBackend::Audio => (0, 0),
        }
    }

    pub fn stop(&mut self) {
        match self {
            PlaybackBackend::Midi { ctx } => {
                let mut ctx = ctx.write();
                ctx.stop();
            }
            PlaybackBackend::Audio => {}
        }
    }

    pub fn get_backend(&self) -> Arc<RwLock<MidiContext>> {
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
    Arc<RwLock<PlaybackContext>>,
    Sender<()>,
) {
    // crossbeam channel for sending messages to the playback thread
    let (tx, rx) = crossbeam::channel::unbounded();

    let context = PlaybackContext::new();

    let arc = Arc::new(RwLock::new(context));

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
                            let mut l = arc2.read();

                            if let Some(time) = dur {}
                            if let Some(backend) = &l.backend {
                                debug!("position: {:?}", backend.get_time());
                                backend.get_backend().write().seek = true;
                                backend.get_backend().write().midi_tick = position;
                            }

                            // println!("position: {:?}", position);
                        }
                        PlaybackEvent::Total(total) => {
                            println!("total: {:?}", total);
                        }
                        PlaybackEvent::Pause => {
                            println!("pause");
                            midi.send(MidiMessage::ClearNotes).unwrap();
                            mptx2.send(()).unwrap();
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
                                if let Some(back) = arc3.read().backend.as_ref() {
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
                            let mut l = arc3.write();
                            if let Some(s) = l.backend.as_mut() {
                                s.stop();
                            }
                        }
                        PlaybackEvent::Exit => {
                            println!("exit");
                            closing = true;
                        }
                    }
                } else {
                    warn!("Unable to recieve message");
                }
            }
        }
    });

    (tx, Arc::clone(&arc), mptx)
}

pub fn msg_reciever() -> (Sender<PlaybackContext>, Receiver<PlaybackContext>) {
    crossbeam::channel::bounded(0)
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
