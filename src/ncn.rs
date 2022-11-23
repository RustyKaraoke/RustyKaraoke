//! NCN File parser
//! This module contains the parser for NCN files, rewritten and refactored from tick.rs

use encoding::{all::WINDOWS_874, decode, DecoderTrap, Encoding};
use log::trace;
use std::{fmt::Display, fs::File, io::Read, path::Path};

/// NCN .lyr format
/// the LYR format is in plain text, usually encoded with TIS-620 (Windows-874)
/// the format is as follows:
///
/// The first 4 lines are metadata, and are ignored on playback
/// The first line is the song title, the second is the artist, the third is the song key. The fourth line is kept blank as a separator.
///
/// ```txt
/// Don't stop me now
/// Queen
/// F
///
/// ...
/// ```
pub struct NcnLyrics {
    pub title: String,
    pub author: String,
    pub key: String,
    pub lyrics: String,
}

// should we move this somewhere else
impl Display for NcnLyrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\n{}\n{}\n\n{}",
            self.title, self.author, self.key, self.lyrics
        )
    }
}

impl NcnLyrics {
    /// New lyrics object
    pub fn new(title: String, author: String, key: String, lyrics: String) -> Self {
        Self {
            title,
            author,
            key,
            lyrics,
        }
    }

    /// Read lyrics from a file
    pub fn read(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(path).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();

        let (res, enc) = decode(&data, DecoderTrap::Replace, WINDOWS_874);

        trace!("Encoding: {:?}", enc.name());

        let file = res?;

        let title = file.lines().next().unwrap().to_string();
        let author = file.lines().nth(1).unwrap().to_string();
        let key = file.lines().nth(2).unwrap().to_string();

        let lyrics = file.lines().skip(4).collect::<Vec<&str>>().join("\n");

        Ok(Self::new(title, author, key, lyrics))
    }
}

// NCN .cur format types
/// The MIDI time tick to scroll a character
#[derive(Debug, Clone, Copy, Ord, PartialEq, PartialOrd, Eq)]
pub struct CurTick {
    pub tick: u32,
}

impl CurTick {
    /// New CurTick
    pub fn new(tick: u32) -> Self {
        Self { tick }
    }
}

#[derive(Debug, Clone, Ord, PartialEq, PartialOrd, Eq)]
/// CUR file data
pub struct NcnCursor {
    pub data: Vec<CurTick>,
    pub pos: usize,
}

impl From<Vec<CurTick>> for NcnCursor {
    fn from(data: Vec<CurTick>) -> Self {
        Self { data, pos: 0 }
    }
}

impl From<Vec<u8>> for NcnCursor {
    fn from(data: Vec<u8>) -> Self {
        let mut ticks = Vec::new();

        for i in (0..data.len()).step_by(4) {
            // try [0; 2] as result
            let mut bytes = [0; 2];
            // try clone from slice as Result, unless there's only 1 byte left
            if i + 1 < data.len() {
                bytes.clone_from_slice(&data[i..i + 2]);
            } else {
                bytes[0] = data[i];
            }

            // debug!("bytes: {:?}", bytes);
            let time = u16::from_le_bytes(bytes);
            // funny bitwise trick to convert 2 bytes into u16
            // let time = ((bytes[0] as u16) << 8) | bytes[1] as u16;
            ticks.push(CurTick::new(time as u32));
        }

        Self::from(ticks)
    }
}

impl NcnCursor {
    pub fn new(data: Vec<u32>) -> Self {
        Self {
            data: data.into_iter().map(CurTick::new).collect(),
            pos: 0,
        }
    }

    /// Read cursor from a file
    pub fn read(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(path).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();

        Ok(Self::from(data))
    }

    /// get all the scrolls in the cursor
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// get current scroll position
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// get the scroll at a specific index
    pub fn get(&self, index: usize) -> Option<&CurTick> {
        self.data.get(index)
    }

    /// check midi time for a specific scroll
    pub fn scroll_check(&self, time: u32) -> Option<usize> {
        for (i, tick) in self.data.iter().enumerate() {
            if tick.tick == time {
                return Some(i);
            }
        }

        None
    }

    /// Scroll to a specific index
    pub fn scroll_to(&mut self, index: usize) -> Option<&CurTick> {
        if let Some(tick) = self.data.get(index) {
            self.pos = index;
            Some(tick)
        } else {
            None
        }
    }

    /// Scroll to a specific time
    pub fn scroll(&mut self, time: u32) -> Option<&CurTick> {
        // get the MIDI time from function argument
        // then check for the last scroll that is less than or equal to the time
        let mut ticks = self
            .data
            .iter()
            .filter(|tick| tick.tick <= time)
            .collect::<Vec<&CurTick>>();
        ticks.sort();

        println!("ticks: {:?}", ticks);

        if let Some(tick) = ticks.last() {
            self.pos = self.data.iter().position(|t| *t == **tick).unwrap();
            Some(tick)
        } else {
            None
        }
    }

    /// Get the last scrolled time
    pub fn last_scroll(&self) -> Option<&CurTick> {
        self.data.get(self.pos)
    }

    pub fn next(&self) -> Option<&CurTick> {
        self.data.get(self.pos + 1)
    }

    pub fn prev(&self) -> Option<&CurTick> {
        self.data.get(self.pos - 1)
    }
}

#[test]
fn test_sort() {
    let mut ticks = vec![
        CurTick::new(5),
        CurTick::new(4),
        CurTick::new(1),
        CurTick::new(2),
        CurTick::new(3),
    ];
    ticks.sort();
    assert_eq!(
        ticks,
        vec![
            CurTick::new(1),
            CurTick::new(2),
            CurTick::new(3),
            CurTick::new(4),
            CurTick::new(5)
        ]
    );
}

#[test]
fn test_scroll_pos() {
    let mut cursor = NcnCursor::new(vec![1, 2, 3, 4, 5]);
    // assert_eq!(cursor.pos(), 0);
    cursor.scroll(6);
    assert_eq!(cursor.last_scroll(), Some(&CurTick::new(5)));
}
