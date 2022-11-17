use log::LevelFilter;

fn main() {
    pretty_env_logger::formatted_builder()
        .filter_level(LevelFilter::Debug)
        .init();

    log::debug!("debug");
    println!("Hello, world!");
}
