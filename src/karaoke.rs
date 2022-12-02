/// Central struct where every karaoke file converted to

pub struct Karaoke {
    pub header: KaraokeHeader,
    pub info: KaraokeInfo,
    pub lyrics: String,
    pub cursor: KaraokeCursor,
    pub midi: Vec<u8>,
}

pub struct KaraokeHeader {
    pub signature: String,
    pub version: String,
}

pub enum SongType {
    // I dont know how much song type there are
    Other(String),
    Undefined,
}

pub enum SubtitleType {
    // I dont know how much subtitle type there are
    Other(String),
    Undefined,
}

pub enum KaraokeLanguage {
    Thai,
    English,
    Other(String),
    Undefined,
}

pub struct KaraokeInfo {
    /// ID of the EMK file
    pub code: Option<String>,
    /// Type of EMK file
    pub song_type: SongType,
    /// Subtitle type
    pub subtitle_type: SubtitleType,
    /// Song title
    pub title: String,
    /// Key of the song
    pub key: String,
    /// Artist of the song
    pub author: String,
    /// Language
    pub language: KaraokeLanguage,
    // I'm not sure about any of this, may need to discussed first

    // pub vocal_channel: u8,
    // /// Original file name
    // pub original_file: String,
    // /// Lyric title
    // pub lyric_title: String,
    // /// Start time of the song
    // pub start_time: u32,
    // /// End time of the song
    // pub end_time: u32,
    // /// Tempo of the song
    // pub tempo: u32,
}

pub struct KaraokeCursor {
    pub data: Vec<KaraokeCursorTick>,
    pub pos: usize,
}

impl KaraokeCursor {
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// get current scroll position
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// get the scroll at a specific index
    pub fn get(&self, index: usize) -> Option<&KaraokeCursorTick> {
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
    pub fn scroll_to(&mut self, index: usize) -> Option<&KaraokeCursorTick> {
        if let Some(tick) = self.data.get(index) {
            self.pos = index;
            Some(tick)
        } else {
            None
        }
    }

    /// Scroll to a specific time
    pub fn scroll(&mut self, time: u32) -> Option<&KaraokeCursorTick> {
        // get the MIDI time from function argument
        // then check for the last scroll that is less than or equal to the time
        let mut ticks = self
            .data
            .iter()
            .filter(|tick| tick.tick <= time)
            .collect::<Vec<&KaraokeCursorTick>>();
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
    pub fn last_scroll(&self) -> Option<&KaraokeCursorTick> {
        self.data.get(self.pos)
    }

    pub fn next(&self) -> Option<&KaraokeCursorTick> {
        self.data.get(self.pos + 1)
    }

    pub fn prev(&self) -> Option<&KaraokeCursorTick> {
        self.data.get(self.pos - 1)
    }
}

// Make it generic
impl From<Vec<u32>> for KaraokeCursor {
    fn from(data: Vec<u32>) -> Self {
        Self {
            data: data.into_iter().map(KaraokeCursorTick::new).collect(),
            pos: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Ord, PartialEq, PartialOrd, Eq)]
pub struct KaraokeCursorTick {
    pub tick: u32,
}

impl KaraokeCursorTick {
    fn new(tick: u32) -> KaraokeCursorTick {
        return KaraokeCursorTick { tick };
    }
}

// The same test as last one except with the migrated struct

#[test]
fn test_sort() {
    let mut ticks = vec![
        KaraokeCursorTick::new(5),
        KaraokeCursorTick::new(4),
        KaraokeCursorTick::new(1),
        KaraokeCursorTick::new(2),
        KaraokeCursorTick::new(3),
    ];
    ticks.sort();
    assert_eq!(
        ticks,
        vec![
            KaraokeCursorTick::new(1),
            KaraokeCursorTick::new(2),
            KaraokeCursorTick::new(3),
            KaraokeCursorTick::new(4),
            KaraokeCursorTick::new(5)
        ]
    );
}

#[test]
fn test_scroll_pos() {
    let mut cursor = KaraokeCursor::from(vec![1, 2, 3, 4, 5]);
    // assert_eq!(cursor.pos(), 0);
    cursor.scroll(6);
    assert_eq!(cursor.last_scroll(), Some(&KaraokeCursorTick::new(5)));
}
