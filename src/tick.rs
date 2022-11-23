use std::{fs::File, io::{Read, Write}};

use encoding::{all::WINDOWS_874, Encoding, DecoderTrap};
use log::debug;
/// MIDI Ticks for CUR and KAR timings
// TODO: Move this to a separate crate
use midly::Timing;


pub struct CurTick {
    // each CUR character is a sequence of 2 bytes
    pub time: u16,
}


/// Raw CUR data
pub struct CurData {
    pub data: Vec<u8>,
}


impl CurData {
    /// Parse raw CUR data
    pub fn read(data: Vec<u8>) -> Self {
        Self { data }
    }


    pub fn into_tick(&self) -> Vec<u16> {
        // the CUR file is a sequence of 2-byte characters
        // turn every 2 bytes into a u16

        // for every 2 bytes, turn into a u16
        // the first byte is the BPM, so we can leave it out

        let data = self.data.clone();

        let mut ticks = Vec::new();
        // let bpm = data.remove(0);

        // debug!("BPM: {}", i32::from(bpm));

        debug!("{:?} characters to be scrolled", data.len());

        debug!("data: {:?}", data.len());

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
            ticks.push(time);
        }

        // ticks.sort();

        debug!("{:?}", ticks);
        ticks
    }
}


pub fn scroll(s: char) {
    // for c in s.chars() {
    //     print!("{c}");
    //     std::io::stdout().flush().expect("Flushing to succeed");
    //     // std::thread::sleep(std::time::Duration::from_millis(75));
    // }
    print!("{s}");
    std::io::stdout().flush().expect("Flushing to succeed");
}

pub fn cur_test(tick: u16) {
    // read file
    // let file = std::fs::read("30664.CUR").unwrap();
    let mut file = File::open("30664.CUR").unwrap();

    let mut buf = vec![];
    // read all bytes
    file.read_to_end(&mut buf).unwrap();
    // parse file
    let cur = CurData::read(buf);
    let t = cur.into_tick();


    // read the lyrics file, excluding the first 4 lines
    let mut lyrics_file = File::open("30664.LYR").unwrap();

    let mut buf = Vec::new();
    lyrics_file.read_to_end(&mut buf).unwrap();

    let lyrics = WINDOWS_874.decode(&buf, DecoderTrap::Strict).unwrap();

    // skip the first 4 lines, then turn back into one string
    let lyrics = lyrics.lines().skip(4).collect::<Vec<&str>>().join("\n");

    // println!("{:?}", lyrics);


    debug!("{} characters to be scrolled in lyrics file", lyrics.chars().count());



    // if t.contains(&tick) {
    //     scroll(&lyrics.remove(0).to_string());
    // }


    // for i in 0..20000 {
    //     // wait for 1 millisecond
    //     std::thread::sleep(std::time::Duration::from_millis(1));

    //     // println!("milliseconds: {}", i);
    //     // if i number is in the vector
    //     if t.contains(&(i as u16)) {
    //         // print the character
    //         // println!("{}: {}", i, lyrics.remove(0));
    //         scroll(&lyrics.remove(0).to_string());
    //     }
    // }
}



/// parsed CUR data
pub struct Cur {
    pub bpm: u16,
    pub data: Vec<CurTick>,
}