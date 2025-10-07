use std::sync::mpsc;
use eframe::egui::RichText;
use eframe::egui::{self, TextureHandle, Color32};

use crate::CtrlMessage;
use crate::Ctrl;


#[derive(Default)]
pub struct GUIApp {
    pub tex: Option<TextureHandle>,
    pub w: usize,
    pub h: usize,
    pub frame: u32,
    pub mem_rx: Option<mpsc::Receiver<CtrlMessage>>,
    pub ctrl_tx: Option<mpsc::Sender<CtrlMessage>>,
    rgba: Vec<u8>,
}
impl eframe::App for GUIApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let (w, h) = (800, 480);
        self.w = w; self.h = h;
        self.frame = self.frame.wrapping_add(1);

        // Make a simple animated gradient
        // for y in 0..h {
        //     for x in 0..w {
        //         let i = (y * w + x) * 4;
        //         rgba[i + 0] = ((x as f32 / 800.0) * 255.0) as u8; // R
        //         rgba[i + 1] = ((y as f32 / 480.0) * 255.0) as u8; // G
        //         // rgba[i + 2] = (x as u32) as u8;   // B
        //         rgba[i + 3] = 255;                           // A
        //     }
        // }


        if let Some(rx) = &self.mem_rx {
            while let Ok(msg) = rx.try_recv() {
                // Handle the received CtrlMessage here
                // For example, update rgba or other state
                self.rgba = msg.data.clone();
                // self.rgba = msg.data;
            }
        }

        let img = egui::ColorImage::from_rgba_unmultiplied([w, h], &self.rgba);
        let tex = self.tex.get_or_insert_with(|| {
            ctx.load_texture("pixels", img.clone(), egui::TextureOptions::NEAREST) // NEAREST for crisp pixels
        });
        tex.set(img, egui::TextureOptions::NEAREST); // update each frame

        egui::CentralPanel::default()
            .frame(
                egui::Frame {
                    fill: Color32::from_rgb(25, 15, 25),
                    ..Default::default()
                }
            ) // remove default padding & background
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                ui.image(&*tex);
                ui.heading(
                    RichText::new("Temp VGA-TextMode")
                        .color(Color32::WHITE)
                        .size(24.0)
                );
                ui.add_space(10.0); // 10 px vertical space
                ui.label(
                    RichText::new( format!("Frame: {} \n", self.frame))
                        .monospace()
                        .color(Color32::LIGHT_GREEN)
                        .size(18.0)
                );
            });

        ctx.request_repaint(); // we’re animating
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(tx) = &self.ctrl_tx {
            let _ = tx.send(CtrlMessage { command: Ctrl::Stop, data: Vec::new() });
        }
    }
}
