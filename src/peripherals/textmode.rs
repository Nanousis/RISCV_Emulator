use std::sync::mpsc;

use eframe::egui::debug_text::print;

use crate::{bus::{Addr, Device}, Ctrl, CtrlMessage};
const _FONT_HEIGHT: u32 = 16;
const _FONT_WIDTH: u32 = 8;
const _TEXT_WIDTH: u32 = 64;
const _TEXT_HEIGHT: u32 = 19;

pub struct TextMode {
    pub mem_tx: mpsc::Sender<CtrlMessage>,
    width: u32,
    height: u32,
    screen_data: Vec<u8>,
    text_data: Vec<u8>,
}
impl TextMode {
    pub fn new(mem_tx: mpsc::Sender<CtrlMessage>) -> Self {
        print!("Initiated VGA-TextMode\n");
        let width = 800;
        let height = 480;
        Self {
            mem_tx,
            width,
            height,
            screen_data: vec![0u8; (width * height * 4) as usize], // 4MB for 800x480 resolution
            text_data: vec![0u8; (_TEXT_WIDTH * _TEXT_HEIGHT * 2) as usize], // 2 bytes per character (ASCII + attribute)
        }
    }
    fn convert_vec_to_string(&self) -> String {
        let mut result = String::new();
        let mut i = 0;
        for chunk in self.text_data.chunks(2) {
            if let Some(&char_byte) = chunk.get(0) {
                if char_byte == 0 {
                    result.push(' '); // Replace null bytes with space
                    if i % 64 == 64 - 1 {
                        result.push('\n'); // New line after 64 characters
                    }
                } else if let Some(ch) = char::from_u32(char_byte as u32) {
                    result.push(ch);
                } else {
                    result.push('?'); // Replace invalid characters with '?'
                }
            }
            i += 1;
        }
        result
    }
}


impl Device for TextMode {
    fn read(&mut self, size: u8, addr: Addr) -> u32 {
        let o = addr as usize;
        match size {
            1 => self.text_data[o] as u32,
            2 => {
                let bytes: [u8; 2] = self.text_data[o..o + 2]
                    .try_into()
                    .expect("RAM: 16-bit read OOB");
                u16::from_le_bytes(bytes) as u32
            }
            4 => {
                let bytes: [u8; 4] = self.text_data[o..o + 4]
                    .try_into()
                    .expect("RAM: 32-bit read OOB");
                u32::from_le_bytes(bytes)
            }
            _ => panic!("Invalid read size: {size}"),
        }
    }

    fn write(&mut self, size: u8, addr: Addr, value: u32) -> Result<(), ()> {
        let o = addr as usize;
        match size {
            1 => {
                self.text_data[o] = value as u8;
            }
            2 => {
                let bytes = (value as u16).to_le_bytes(); // 2 bytes
                let str = self.convert_vec_to_string();
                let _ = self.mem_tx.send(CtrlMessage { command: Ctrl::Data, data: str });
                self.text_data[o..o + 2].copy_from_slice(&bytes);
            }
            4 => {
                let bytes = value.to_le_bytes(); // 4 bytes
                self.text_data[o..o + 4].copy_from_slice(&bytes);
            }
            _ => return Err(()),
        }
        Ok(())
    }
}
