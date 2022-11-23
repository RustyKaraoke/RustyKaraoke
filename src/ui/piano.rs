//! Piano widget for egui

use eframe::epaint::RectShape;
use egui::{Rounding, Widget};

use crate::midi::{MidiDisplay, Track};

pub struct Piano {
    pub state: Track,
}

impl Widget for Piano {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let Piano { state } = self;

        let (id, rect) = ui.allocate_space(egui::vec2(0.0, 20.0));

        // new ui with 128 keys

        // ui.


        // let r = ui.allocate_response(egui::vec2(1000.0, 200.0), egui::Sense::hover());
        ui.horizontal(|ui| {
            // vertical spacing between keys should be 0
            // ui.spacing_mut().icon_spacing = 0.0;

            ui.spacing_mut().item_spacing.x = 1.0;
            ui.spacing_mut().item_spacing.y = 1.0;
            for i in 0..88 {

                let rect = ui.allocate_response(egui::vec2(5.0, 50.0), egui::Sense::click_and_drag());

                let key = state.notes.iter().find(|n| n.note == i);
                // 0 spacing between rects
                let color = match key {
                    Some(_) => egui::Color32::WHITE,
                    None => egui::Color32::BLACK,
                };
                // let's paint the key
                ui.painter().add(RectShape {
                    rect: rect.rect,
                    rounding: Rounding::none(),
                    // corner_radius: 0.0,
                    fill: color,
                    stroke: egui::Stroke::none(),
                });

            }
            // let's allocate a space for a key
        }).response
        // egui::widgets::Button::new("test").ui(ui);
    }
}
