//! Timing module for playback progress.

use std::sync::{Arc, atomic::AtomicU64};

use chrono::Duration;
use nodi::Connection;
// should i make this a singleton?
// or should i make it a struct that is passed around?


pub struct PlaybackContext {
    pub position: Option<Duration>,
    pub total: Option<Duration>,
    pub backend: Option<PlaybackBackend>,
}

pub enum PlaybackBackend {
    Midi,
    Audio,
}

impl PlaybackContext {
    pub fn new() -> Self {
        Self {
            position: None,
            total: None,
            backend: None,
        }
    }
}

// initiate shared atomic for playback

// TODO: Mutex time
// fn playback() {
//     let mut playback = Context::new();


//     let mut p = Arc::new(playback);
//     p.playback.backend = Some(PlaybackBackend::Midi);


//     // edit the shared atomic
//     std::thread::spawn(move || {
//         loop {
//             std::thread::sleep(std::time::Duration::from_millis(1000));
//         }
//     });


// }


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