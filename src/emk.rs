use log::debug;
use nodi::Sheet;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::{fs::File, io::Read};
use flate2::read::ZlibDecoder;

use md5::{Digest, Md5};

use crate::ncn::{NcnCursor, NcnLyrics};

// EMK magic xor key, works for all EMK files.
// credits to alula for figuring this out
// AFF24C9CE9EA9943
static EMK_MAGIC: u64 = 0xAFF24C9CE9EA9943;

static MAGIC: u64 = 0x2e53464453;

fn xor(data: Vec<u8>, key: u64) -> Vec<u8> {
    println!("XORing with key: {:X}", key);
    let key = key.to_be_bytes().to_vec();
    println!("key: {:X?}", key);
    let mut data = data;
    for i in 0..data.len() {
        // println!("data: {:?}", data[i]);
        data[i] ^= key[i % key.len()];
        // println!("data xor: {:?}", data[i]);
    }

    let magic = vec![0x2e, 0x53, 0x46, 0x44, 0x53];
    if data[0..magic.len()] != magic {
        println!("Invalid magic");
    }

    data
}

pub struct EmkHeader {
    pub signature: String,
    pub version: String,
}

pub struct EmkInfo {
    /// ID of the EMK file
    pub code: String,
    /// Type of EMK file
    // TODO: make this an enum
    pub song_type: String,
    /// Subtitle type
    // TODO: make this an enum
    pub subtitle_type: String,
    /// Song title
    pub title: String,
    /// Key of the song
    pub key: String,
    /// Artist of the song
    pub artist: String,
    /// Language
    pub language: String,
    /// MIDI channel with the vocals
    pub vocal_channel: u8,
    /// Original file name
    pub original_file: String,
    /// Lyric title
    pub lyric_title: String,
    /// Start time of the song
    pub start_time: u32,
    /// End time of the song
    pub end_time: u32,
    /// Tempo of the song
    pub tempo: u32,
}

pub struct Emk {
    pub header: EmkHeader,
    pub info: EmkInfo,
    pub lyrics: NcnLyrics,
    pub cursor: NcnCursor,
    pub midi: Vec<u8>,
}
#[derive(Debug, Clone, Copy, FromPrimitive)]
enum Tag {
    Byte = 2,
    Short = 3,
    Int = 4,
    String = 6,
}
#[derive(Debug, Clone)]
enum TagOut {
    Byte(u8),
    Short(u16),
    Int(u32),
    String(String),
}

impl TagOut {
    pub fn to_string(&self) -> String {
        match self {
            TagOut::Byte(b) => b.to_string(),
            TagOut::Short(s) => s.to_string(),
            TagOut::Int(i) => i.to_string(),
            TagOut::String(s) => s.to_string(),
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            TagOut::Byte(b) => *b,
            TagOut::Short(s) => *s as u8,
            TagOut::Int(i) => *i as u8,
            TagOut::String(s) => s.parse::<u8>().unwrap(),
        }
    }


    pub fn to_u16(&self) -> u16 {
        match self {
            TagOut::Byte(b) => *b as u16,
            TagOut::Short(s) => *s,
            TagOut::Int(i) => *i as u16,
            TagOut::String(s) => s.parse::<u16>().unwrap(),
        }
    }


    pub fn to_u32(&self) -> u32 {
        match self {
            TagOut::Byte(b) => *b as u32,
            TagOut::Short(s) => *s as u32,
            TagOut::Int(i) => *i,
            TagOut::String(s) => s.parse::<u32>().unwrap(),
        }
    }



}

struct EmkReader {
    data: Vec<u8>,
    header: Vec<u8>,
    pos: usize,
}

impl EmkReader {
    fn new(data: Vec<u8>) -> Self {
        let data = xor(data, EMK_MAGIC);

        // header start and end is u64 little endian
        let header_pos = u64::from_le_bytes(data[0x22..0x22 + 8].try_into().unwrap()) as usize;
        println!("header start: {}", header_pos);
        let header_end = u64::from_le_bytes(data[0x2a..0x2a + 8].try_into().unwrap()) as usize;
        println!("header end: {}", header_end);

        let header = data[header_pos..header_end].to_vec();
        println!("header len: {}", header.len());

        println!("header: {:?}", header);
        Self {
            data,
            header,
            pos: 0,
        }
    }

    fn check_magic(&mut self, magic: &[u8]) {
        let data = self.header[self.pos..self.pos + magic.len()].to_vec();
        if data != magic {
            panic!("Invalid magic");
        }
        // Oh yeah, we need to skip magic bytes
        self.pos += magic.len();
    }

    fn skip(&mut self, n: usize) {
        self.pos += n;
    }

    fn read_byte(&mut self) -> u8 {
        let byte = self.header[self.pos];
        self.pos += 1;

        byte
    }

    fn read_u16(&mut self) -> u16 {
        let res = u16::from_le_bytes(self.header[self.pos..self.pos + 2].try_into().unwrap());
        self.pos += 2;
        res
    }

    fn read_u32(&mut self) -> u32 {
        let res = u32::from_le_bytes(self.header[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        res
    }
    fn read_string(&mut self) -> String {
        let len = self.read_byte() as usize;
        let str = String::from_utf8(self.header[self.pos..self.pos + len].to_vec()).unwrap();
        self.pos += len;
        str
    }

    fn read_tag(&mut self) -> TagOut {
        let byte = self.read_byte();

        // println!("Reading tag: {}", byte);

        let tag: Option<Tag> = FromPrimitive::from_u8(byte);
        match tag {
            Some(Tag::Byte) => {
                let v = self.read_byte();
                // println!("Byte: {}", v);
                TagOut::Byte(v)
            }
            Some(Tag::Short) => {
                let v = self.read_u16();
                // println!("Short: {}", v);
                TagOut::Short(v)
            }
            Some(Tag::Int) => {
                let v = self.read_u32();
                // println!("Int: {}", v);
                TagOut::Int(v)
            }
            Some(Tag::String) => {
                let v = self.read_string();
                // println!("String: {}", v);
                TagOut::String(v)
            }

            None => todo!(),
        }
    }

    fn read_header(&mut self) {
        let magic = [0x53, 0x46, 0x44, 0x53];
        // self.decrypt();
        while self.pos < self.header.len() {
            println!("=== Header ===");
            self.check_magic(&magic);
            let tag = self.read_tag();
            println!("Tag: {:?}", tag);
            let uncompressed_size = self.read_tag();
            println!("Uncompressed size: {:?}", uncompressed_size);
            let _unk2 = self.read_tag();
            println!("Unk2: {:?}", _unk2);
            let data_begin = self.read_tag().to_u16();
            println!("Data begin: {:?}", data_begin);
            let data_end = self.read_tag().to_u16();
            println!("Data end: {:?}", data_end);
            let _unk5 = self.read_tag();
            println!("Unk5: {:?}", _unk5);
            let _unk6 = self.read_tag();
            println!("Unk6: {:?}", _unk6);
            // next 16 bytes are MD5 hash of the compressed data
            let md5_hash = {
                let res = self.data[self.pos..self.pos + 16].to_vec();
                self.pos += 16;
                res
            };
            // get md5 hash
            // self.skip(0x10);
            println!("MD5 hash: {:?}", md5_hash);
            let _unk7 = self.read_tag();
            println!("Unk7: {:?}", _unk7);
            let _unk8 = self.read_tag();
            println!("Unk8: {:?}", _unk8);

            // compressed data
            let mut hasher = Md5::new();
            let compressed_data = self.data[data_begin as usize..data_end as usize].to_vec();

            // check md5 hash

            // ? md5 hash is wrong for some reason
            let raw_data = {
                let mut buf = Vec::new();
                let mut decoder = ZlibDecoder::new(compressed_data.as_slice());
                decoder.read_to_end(&mut buf).unwrap();
                buf
            };
            hasher.update(&raw_data.as_slice());
            let hash = hasher.finalize_reset();
            println!("Hash: {:?}", hash);
            println!("Embedded Hash: {:?}", md5_hash);

            if let TagOut::String(s) = tag {
                if let "HEADER" = s.as_str() {
                    println!("--- HEADER ---");
                    println!("{}", String::from_utf8(raw_data).unwrap());
                    println!("--- END HEADER ---");
                }
            }
        }
    }
}
#[test]
fn read_emk() {
    let mut file = File::open("/home/cappy/000001.emk").unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();

    // header_pos is 0x22
    // let header: Vec<u8> = data[0x22..0x2a].to_vec();

    {
        let mut reader = EmkReader::new(data);
        reader.read_header();
    }
}
