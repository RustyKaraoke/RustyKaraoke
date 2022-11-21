mod midi;
mod ncn;
mod tick;
mod time;
use std::sync::Arc;

use eframe::{run_native, App};
use egui::{CentralPanel, Frame, RichText, SidePanel, TopBottomPanel, Ui, ImageButton};
use log::LevelFilter;
use parking_lot::Mutex;
use time::{Context, PlaybackContext};

struct Frontend {
    pub msg: crossbeam::channel::Sender<time::PlaybackEvent>,
    pub context: Arc<Mutex<PlaybackContext>>,
}

impl App for Frontend {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        frame.set_window_title("RustyKaraoke");
        // if cursor on window, print cursor position
        // if ctx.input().pointer.is_moving() {
        //     let pos = ctx.input().pointer.hover_pos();
        //     println!("Cursor: {:?}", pos);
        // }
        fn sidebar_ui(ui: &mut Ui) {
            ui.heading("Side Panel");
            ui.label("This is a side panel");
            ui.label("It can be used to show extra information");
            ui.label("It can be closed by clicking the button below");

            // ui.set_visible(false);
            if ui.button("Close").clicked() {
                // ui.close_side_panel();
                ui.set_visible(false);
            }
        }


        let mut side = SidePanel::right("side_panel").show(ctx, sidebar_ui);
        // frame.set_window_size(egui::vec2(1280.0, 720.0));
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // ui.heading("RustyKaraoke");
                ui.menu_button("File", |ui| {
                    if ui.button("Close the menu").clicked() {
                        ui.close_menu();
                    }

                    if ui.button("Quit").clicked() {
                        std::process::exit(0);
                    }
                });
                ui.separator();
                ui.spacing();
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Max), |ui| {
                        ui.label("Volume");
                        ui.add(egui::Slider::new(&mut 0.0, 0.0..=1.0).text("Volume"));
                        // button to toggle sidebar
                        if ui.button("Toggle Sidebar").clicked() {
                            // side.toggle();

                            // hide sidebar
                        }
                    });
                });
                ui.button("Open");
            });
            // ui.spinner();
        });
        
        // new window
        
        // side panel as an overlay to the current ui
        
        // button to toggle the side panel
        let mut num = 0.5;

        egui::Window::new("Playback")
            .fixed_size(egui::vec2(500.0, 200.0))
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Play").clicked() {
                        // run await
                        // let c = Arc::clone(&self.context);
                        // if context.playback.player.is_none() {
                        //     context.playback.backend = Some(time::PlaybackBackend::Midi {
                        //         cursor: None,
                        //     });
                        //     let h = tokio::spawn(async move {
                        //         midi::run(c).await;
                        //     });

                        //     context.playback.player = Some(h);
                        // }
                        // await the handle but the function is not awaitable

                        self.msg.send(time::PlaybackEvent::Play).unwrap();
                    }
                    if ui.button("Pause").clicked() {
                        self.msg.send(time::PlaybackEvent::Pause).unwrap();
                    }
                    // button with icon
                    ui.add(ImageButton::new(
                        egui::TextureId::default(),
                        egui::vec2(16.0, 16.0),
                    ));
                    if ui.button("Stop").clicked() {
                        // let mut context = self.context.lock();
                        self.msg.send(time::PlaybackEvent::Stop).unwrap();
                    }
                });


                if ui.add(egui::Slider::new(&mut num, 0.0..=1.0).smart_aim(false).show_value(false)).changed() {
                    // num = num
                    self.persist_egui_memory();
                    println!("Slider: {}", num);
                };
            });

        CentralPanel::default().show(ctx, |ui| {
            ui.label("Hello World!");
            ui.code(RichText::new("aaa").code());
            // let ctx = self.context.lock();
            ui.code(format!("{:#?}", *self.context.lock()));
            ui.code(format!("{:#?}", num));
            // text centered
            ui.vertical_centered(|ui| {
                ui.heading("RustyKaraoke");
            });
        });
        // ctx.set_debug_on_hover(true);

        ctx.request_repaint();
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    fn on_close_event(&mut self) -> bool {
        println!("Closing");
        // drop(self.msg.to_owned());
        self.msg.send(time::PlaybackEvent::Exit).unwrap();
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

    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1280.0, 720.0)),
        min_window_size: Some(egui::vec2(800.0, 600.0)),
        resizable: true,
        follow_system_theme: false,
        ..Default::default()
    };

    let (tx, backrx) = crate::time::msg_send();

    let app = Frontend {
        msg: tx,
        context: backrx,
    };

    // use funny crossbeam channel to send messages to the main thread
    // let (tx, rx) = crossbeam::channel::bounded(1);

    run_native("RustyKaraoke", native_options, Box::new(|_| Box::new(app)));
}

fn init_context() -> Arc<Mutex<Context>> {
    let context = Context::new();
    Arc::new(Mutex::new(context))
}
