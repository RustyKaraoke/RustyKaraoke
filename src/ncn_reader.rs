// Literally the same as NCN file except it is migrated to Karaoke

use std::{error::Error, fs::File, io::Read, path::Path};

use encoding::{all::WINDOWS_874, decode, DecoderTrap};

use crate::karaoke::{
    KaraokeCursor, KaraokeCursorTick, KaraokeInfo, KaraokeLanguage, SongType, SubtitleType,
};

struct NcnLyricsReader {
    pub title: String,
    pub author: String,
    pub key: String,
    pub lyrics: String,
}

impl NcnLyricsReader {
    fn from_path(path: &Path) -> Result<Self, Box<dyn Error>> {
        let mut file = File::open(path).unwrap();
        let mut file_data = Vec::new();
        file.read_to_end(&mut file_data).unwrap();

        let (result, enconding) = decode(&file_data, DecoderTrap::Replace, WINDOWS_874);

        let result_data = result?;
        let title = result_data.lines().next().unwrap().to_string();
        let author = result_data.lines().nth(1).unwrap().to_string();
        let key = result_data.lines().nth(2).unwrap().to_string();

        let lyrics = result_data
            .lines()
            .skip(4)
            .collect::<Vec<&str>>()
            .join("\n");

        return Ok(NcnLyricsReader {
            title,
            author,
            key,
            lyrics,
        });
    }

    fn get_lyrics(&self) -> String {
        return String::from(&self.lyrics);
    }

    fn get_info(&self) -> KaraokeInfo {
        return KaraokeInfo {
            code: None,
            song_type: SongType::Undefined,
            subtitle_type: SubtitleType::Undefined,
            title: String::from(&self.title),
            key: String::from(&self.key),
            author: String::from(&self.author),
            language: KaraokeLanguage::Undefined,
        };
    }
}

struct NcnCursorReader {
    pub data: Vec<u8>,
}

impl NcnCursorReader {
    fn from_path(path: &Path) -> Result<Self, Box<dyn Error>> {
        let mut file = File::open(path).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();

        return Ok(Self { data });
    }

    fn get_cursor(&self) -> KaraokeCursor {
        let mut ticks = Vec::new();

        for i in (0..self.data.len()).step_by(4) {
            // try [0; 2] as result
            let mut bytes = [0; 2];
            // try clone from slice as Result, unless there's only 1 byte left
            if i + 1 < self.data.len() {
                bytes.clone_from_slice(&self.data[i..i + 2]);
            } else {
                bytes[0] = self.data[i];
            }

            // debug!("bytes: {:?}", bytes);
            let time = u16::from_le_bytes(bytes);
            // funny bitwise trick to convert 2 bytes into u16
            // let time = ((bytes[0] as u16) << 8) | bytes[1] as u16;
            ticks.push(KaraokeCursorTick { tick: time as u32 });
        }

        return KaraokeCursor {
            data: ticks,
            pos: 0,
        };
    }
}
