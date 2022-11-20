mod tick;
mod ncn;
mod midi;
use eframe::{App, run_native};
use egui::{CentralPanel, TopBottomPanel};
use log::LevelFilter;


struct Frontend;


impl App for Frontend {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        frame.set_window_title("RustyKaraoke");
        // frame.set_window_size(egui::vec2(1280.0, 720.0));
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.heading("RustyKaraoke");
            ui.spinner();
        });
        CentralPanel::default().show(ctx, |ui| {
            ui.label("Hello World!");
        });
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    fn on_close_event(&mut self) -> bool {
        true
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {}

    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }

    fn max_size_points(&self) -> egui::Vec2 {
        egui::Vec2::INFINITY
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> egui::Rgba {
        // NOTE: a bright gray makes the shadows of the windows look weird.
        // We use a bit of transparency so that if the user switches on the
        // `transparent()` option they get immediate results.
        egui::Color32::from_rgba_unmultiplied(12, 12, 12, 180).into()

        // _visuals.window_fill() would also be a natural choice
    }

    fn persist_native_window(&self) -> bool {
        true
    }

    fn persist_egui_memory(&self) -> bool {
        true
    }

    fn warm_up_enabled(&self) -> bool {
        false
    }

    fn post_rendering(&mut self, _window_size_px: [u32; 2], _frame: &eframe::Frame) {}

}

#[tokio::main]
async fn main() {
    pretty_env_logger::formatted_builder()
        .filter_level(LevelFilter::Debug)
        .init();

    // log::debug!("debug");
    // println!("Hello, world!");

    // println!("{}", load_lyrics().unwrap());

    // cur_test();


    // tokio::spawn(async {
    //     midi::run().await;
    // }).await.unwrap();


    // egui widget
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1280.0, 720.0)),
        min_window_size: Some(egui::vec2(800.0, 600.0)),
        resizable: true,
        follow_system_theme: false,
        ..Default::default()
    };

    let app = Frontend;
    run_native("RustyKaraoke", native_options, Box::new(|_| Box::new(app)));
}
