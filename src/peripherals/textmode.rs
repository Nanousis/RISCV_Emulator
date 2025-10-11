
use std::sync::{Arc, RwLock}; // or Mutex if you prefer

use crate::bus::{Addr, Device};

use crate::constants::*;

struct Attribute {
    fg: [u8; 3],
    bg: [u8; 3],
    // blink: bool,
}

pub type ScreenHandle = Arc<RwLock<Vec<u8>>>; // RGBA8888 buffer

pub struct TextMode {
    width: u32,
    height: u32,
    pub screen_data: ScreenHandle,
    text_data: Vec<u8>,
}
impl TextMode {
    pub fn new() -> Self {
        println!("Initiated VGA-TextMode");
        let pixels = SCREEN_WIDTH * SCREEN_HEIGHT * 4;
        Self {
            width: SCREEN_WIDTH as u32,
            height: SCREEN_HEIGHT as u32,
            screen_data: Arc::new(RwLock::new(vec![255; pixels])),
            text_data: vec![0u8; (_TEXT_WIDTH * _TEXT_HEIGHT * 2) as usize], // 2 bytes per character (ASCII + attribute)
        }
    }
    fn read_attribute(&self, attribute: u8) -> Attribute {
        let fg: [u8; 3] = match attribute & 0xF {
            0x0 => { [0,0,0] }          // Black
            0x1 => { [127,0,0] }        // Red
            0x2 => { [0,127,0] }        // Green
            0x3 => { [127,127,0] }      // Yellow
            0x4 => { [0,0,127] }        // Blue
            0x5 => { [127,0,127] }      // Magenta
            0x6 => { [0, 127, 127] }    // Cyan
            0x7 => { [64,64,64] }       // Dark gray
            0x8 => { [128,128,128] }    // Light gray
            0x9 => { [255, 0, 0] }      // Intense Red
            0xA => { [0, 255, 0] }      // Intense Green
            0xB => { [255, 255, 0] }    // Intense Yellow
            0xC => { [0, 0, 255] }      // Intense Blue
            0xD => { [255, 0, 255] }    // Intense Magenta
            0xE => { [0, 255, 255] }    // Intense Cyan
            0xF => { [255, 255, 255] }  // Intense White
            _ => { [255, 255, 255] }    // Default to white
        };
        let bg: [u8; 3]  = match (attribute >> 4) & 0x7 {
            0x0 => [0, 0, 0],         // Black
            0x1 => [255, 0, 0],       // Red
            0x2 => [0, 255, 0],       // Green
            0x3 => [255, 255, 0],     // Yellow
            0x4 => [0, 0, 255],       // Blue
            0x5 => [255, 0, 255],     // Magenta
            0x6 => [0, 255, 255],     // Cyan
            0x7 => [255, 255, 255],   // White
            _ => [0, 0, 0],           // Default to black
        };
        Attribute { fg, bg }
    }
    pub fn handle(&self) -> ScreenHandle {
        Arc::clone(&self.screen_data)
    }

    fn convert_vec_to_img(&self) -> Vec<u8> {
        let mut img_data = vec![0u8; (self.width * self.height * 4) as usize]; // RGBA for each pixel
        for row in 0.._TEXT_HEIGHT {
            for col in 0.._TEXT_WIDTH {
                let char_index = ((row * _TEXT_WIDTH + col) * 2) as usize;
                let char_byte = self.text_data[char_index];
                // let attr_byte = self.text_data[char_index];
                let attribute = self.read_attribute(self.text_data[char_index + 1]);
                let fg_color = attribute.fg;
                let bg_color = attribute.bg;
                let glyph = if char_byte < 128 {
                    &FONT[char_byte as usize]
                } else {
                    &FONT[0] // Use space for unsupported characters
                };

                for (glyph_row, glyph_byte) in glyph.iter().enumerate() {
                    for bit in 0..8 {
                        let pixel_on = (glyph_byte >> (7 - bit)) & 1 == 1;
                        let color = if pixel_on { fg_color } else { bg_color };

                        let x = col * 8 + bit;
                        let y = row * 16 + glyph_row as u32;
                        if x * 2 < self.width && y * 2 < self.height {
                            let pixel_index = ((y * 2 * self.width + x * 2) * 4) as usize;
                            // Set pixel color in RGBA format for 2x horizontal and vertical scaling
                            // Top-left
                            img_data[pixel_index] = color[0];
                            img_data[pixel_index + 1] = color[1];
                            img_data[pixel_index + 2] = color[2];
                            img_data[pixel_index + 3] = 255;
                            // Top-right
                            img_data[pixel_index + 4] = color[0];
                            img_data[pixel_index + 5] = color[1];
                            img_data[pixel_index + 6] = color[2];
                            img_data[pixel_index + 7] = 255;
                            // Bottom-left
                            let bottom_pixel_index = pixel_index + self.width as usize * 4;
                            img_data[bottom_pixel_index] = color[0];
                            img_data[bottom_pixel_index + 1] = color[1];
                            img_data[bottom_pixel_index + 2] = color[2];
                            img_data[bottom_pixel_index + 3] = 255;
                            // Bottom-right
                            img_data[bottom_pixel_index + 4] = color[0];
                            img_data[bottom_pixel_index + 5] = color[1];
                            img_data[bottom_pixel_index + 6] = color[2];
                            img_data[bottom_pixel_index + 7] = 255;
                        }
                    }
                }
            }
        }
        img_data
    }
    #[allow(dead_code)]
    fn convert_vec_to_string(&self) -> String {
        let mut result = String::new();
        for (i, chunk) in self.text_data.chunks(2).enumerate() {
            if let Some(&char_byte) = chunk.first() {
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
                // let str = self.convert_vec_to_string();
                if self.text_data[o..o + 2] != bytes {
                    self.text_data[o..o + 2].copy_from_slice(&bytes);
                    let data = self.convert_vec_to_img();        // Vec<u8> with RGBA
                    if let Ok(mut screen) = self.screen_data.write() {
                        screen.copy_from_slice(&data);
                    }
                }
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
