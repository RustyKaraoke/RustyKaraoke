mod tick;
mod midi;

use tick::cur_test;

use std::{error::Error, fs::File, io::Read};

use log::LevelFilter;
use encoding::{all::WINDOWS_874, DecoderTrap, Encoding};

#[tokio::main]
async fn main() {
    pretty_env_logger::formatted_builder()
        .filter_level(LevelFilter::Debug)
        .init();

    // log::debug!("debug");
    // println!("Hello, world!");

    // println!("{}", load_lyrics().unwrap());

    // cur_test();


    tokio::spawn(async {
        midi::run().await;
    }).await.unwrap();
    // midi::run().await;
}


fn load_lyrics() -> Result<String, Box<dyn Error>> {
    let mut file = File::open("30664.LYR")?;
    // autodetect encoding
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    // decode to utf8
    let lyrics = WINDOWS_874.decode(&buf, DecoderTrap::Strict)?;
    Ok(lyrics)
}