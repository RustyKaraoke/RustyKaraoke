//! Timing module for playback progress.

use std::sync::{
    atomic::{AtomicU64, AtomicUsize},
    Arc,
};

use chrono::Duration;
use crossbeam::channel::{Receiver, Sender};
use log::debug;
use parking_lot::Mutex;
use tokio::task::JoinHandle;

use crate::midi;
// should i make this a singleton?
// or should i make it a struct that is passed around?

#[derive(Debug, Copy, Clone)]
pub struct PlaybackContext {
    pub position: Option<Duration>,
    pub total: Option<Duration>,
    pub backend: Option<PlaybackBackend>,
    // pub player: Option<JoinHandle<()>>,
    pub paused: bool,
}
#[derive(Debug, Clone, Copy)]
pub enum PlaybackBackend {
    Midi { cursor: Option<usize> },
    Audio,
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
    Position(usize),
    Total(Duration),
    Pause,
    Play,
    Stop,
    Exit,
}

pub fn msg_send() -> (Sender<PlaybackEvent>, Arc<Mutex<PlaybackContext>>) {
    // crossbeam channel for sending messages to the playback thread
    let (tx, rx) = crossbeam::channel::bounded(0);

    let context = PlaybackContext::new();


    let arc = Arc::new(Mutex::new(context));

    let tx2 = tx.clone();

    let mut closing = false;

    let (backtx, backrx) = msg_reciever();

    let arc2 = Arc::clone(&arc);

    // backtx.clone().send(context.clone()).unwrap();
    // recieve messages from the playback thread
    tokio::spawn(async move {
        // this is some very scuffed code
        loop {
            if closing {
                break;
            } else {
                process_ctrl_msg(rx.clone(), &mut closing, tx2.clone(), backtx.clone(), Arc::clone(&arc2)).await;
            }
        }
    });

    (tx, Arc::clone(&arc))
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
) {
    // debug!("Processing message");
    if let Ok(msg) = rx.recv() {
        match msg {
            PlaybackEvent::Backend(backend) => {
                println!("backend: {:?}", backend);
            }
            PlaybackEvent::Position(position) => {
                let mut l = backend.lock();
                l.backend = Some(PlaybackBackend::Midi { cursor: Some(position) });
                // println!("position: {:?}", position);
            }
            PlaybackEvent::Total(total) => {
                println!("total: {:?}", total);
            }
            PlaybackEvent::Pause => {
                println!("pause");
            }
            PlaybackEvent::Play => {
                println!("play");

                tokio::spawn(async move {
                    midi::run(tx, backend).await;
                });
                // midi::run(tx.clone()).await;
            }
            PlaybackEvent::Stop => {
                println!("stop");
            }
            PlaybackEvent::Exit => {
                println!("exit");
                *closing = true;
            }
        }
    }
    // backend_tx.send(*backend).unwrap();
}

#[derive(Debug, Clone, Copy)]
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
