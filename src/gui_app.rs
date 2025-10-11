use std::sync::mpsc;
use std::time::Duration;
use eframe::egui::RichText;
use eframe::egui::{self, TextureHandle, Color32};

use crate::types::{Ctrl, CtrlMessage, ScreenMsg, ScreenType};
use crate::constants::*;

pub struct GUIApp {
    pub tex: Option<TextureHandle>,
    pub w: usize,
    pub h: usize,
    pub screen_rx: Option<mpsc::Receiver<ScreenMsg>>,
    pub uart_rx: Option<mpsc::Receiver<char>>,
    pub ctrl_tx: Option<mpsc::Sender<CtrlMessage>>,
    rgba: Vec<u8>,
    uart_buffer: String,
}
impl GUIApp {
    pub fn send_request_frame(&mut self) {
        if let Some(tx) = &self.ctrl_tx {
            let _ = tx.send(CtrlMessage { command: Ctrl::RequestFrame });
        }
        if let Some(rx) = &self.screen_rx && let Ok(msg) = rx.recv() {
            match msg.screen_type {
                ScreenType::TextMode => {
                    self.rgba = msg.data.clone();
                }
                ScreenType::FrameBuffer => {
                    self.rgba = GUIApp::rgb565_to_rgba8888(&msg.data);
                }
            }
        }
    }
    #[inline]
    pub fn rgb565_to_rgba8888(src: &[u8]) -> Vec<u8> {
        // Expect 2 bytes per pixel
        debug_assert_eq!(src.len(), SCREEN_WIDTH * SCREEN_HEIGHT * 2);

        let mut rgba = vec![0u8; SCREEN_WIDTH * SCREEN_HEIGHT * 4];
        // Iterate two bytes at a time
        let mut si = 0;
        let mut di = 0;
        while si + 1 < src.len() {
            let lo = src[si] as u16;
            let hi = src[si + 1] as u16;
            let pixel = (hi << 8) | lo; // little-endian

            let r5 = ((pixel >> 11) & 0x1F) as u8;
            let g6 = ((pixel >> 5)  & 0x3F) as u8;
            let b5 = ( pixel        & 0x1F) as u8;

            // Expand to 8-bit per channel
            rgba[di    ] = (r5 << 3) | (r5 >> 2);
            rgba[di + 1] = (g6 << 2) | (g6 >> 4);
            rgba[di + 2] = (b5 << 3) | (b5 >> 2);
            rgba[di + 3] = 255;

            si += 2;
            di += 4;
        }
        rgba
    }
}
impl eframe::App for GUIApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.w = SCREEN_WIDTH; self.h = SCREEN_HEIGHT;
        
        let img = egui::ColorImage::from_rgba_unmultiplied([SCREEN_WIDTH, SCREEN_HEIGHT], &self.rgba);
        self.send_request_frame();
        let tex = self.tex.get_or_insert_with(|| {
            ctx.load_texture("pixels", img.clone(), egui::TextureOptions::NEAREST) // NEAREST for crisp pixels
        });
        if let Some(rx) = &self.uart_rx {
            while let Ok(c) = rx.try_recv() {
                // Handle received character
                self.uart_buffer.push(c);
            }
        }
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
                    RichText::new("Uart Log")
                        .color(Color32::WHITE)
                        .size(24.0)
                );
                ui.add_space(10.0); // 10 px vertical space
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.uart_buffer)
                                .code_editor()              // monospace look
                                .desired_width(f32::INFINITY)
                                .desired_rows(6)            // pick how many rows tall it looks
                                .lock_focus(false)
                                .interactive(false)         // read-only
                                .cursor_at_end(true)        // keep caret at end for stick_to_bottom
                        );
                    });
            });
        // Cap the framerate to ~30 FPS. No reason to stress the cpu thread
        ctx.request_repaint_after(Duration::from_millis(33)); // ~30 FPS
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(tx) = &self.ctrl_tx {
            let _ = tx.send(CtrlMessage { command: Ctrl::Stop });
        }
    }
}
impl Default for GUIApp {
    fn default() -> Self {
        let w = SCREEN_WIDTH;
        let h = SCREEN_HEIGHT;
        let mut black = vec![0u8; w * h * 4]; // RGBA, all zero = black
        for i in 0..(w*h) {
            black[i*4 + 3] = 255; // set alpha to fully opaque
        }
        Self {
            tex: None,
            w,
            h,
            screen_rx: None,
            ctrl_tx: None,
            uart_rx: None,
            rgba: black,
            uart_buffer: String::new(),
        }
    }
}
