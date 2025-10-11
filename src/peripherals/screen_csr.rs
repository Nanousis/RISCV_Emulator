
use colored::Colorize;

use crate::bus::{Addr, Device};

pub struct ScreenCsr{
    pub frame_buffer_enabled: bool,
    pub frame_buffer_addr: u32,
}
impl ScreenCsr {
    pub fn new() -> Self {
        Self { frame_buffer_enabled: false, frame_buffer_addr: 0 }
    }
    pub fn is_enabled(&self) -> bool { self.frame_buffer_enabled }
    pub fn fb_addr(&self) -> u32 { self.frame_buffer_addr }
}
impl Device for ScreenCsr {
    fn read(&mut self, size: u8, addr: Addr) -> u32 {
        // Default value for always ready. 
        // TODO: implement proper UART behavior
        if addr < 4 {
            if self.is_enabled() { 1 } else { 0 }
        } else if size == 4 && addr == 4 {
            self.fb_addr()
        } else {
            unimplemented!("{}","THIS SHOULD NEVER HAPPEN: Invalid read on Screen CSR".red().bold());
        }
    }

    fn write(&mut self, size: u8, addr: Addr, value: u32) -> Result<(), ()> {
        if addr < 4 {
            self.frame_buffer_enabled = value != 0;
            Ok(())
        } else if size == 4 && addr == 4 {
            self.frame_buffer_addr = value;
            Ok(())
        } else {
            Err(())
        }
    }
}
