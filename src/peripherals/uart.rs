use std::sync::mpsc;

use crate::bus::{Addr, Device};

pub struct UartNs16550a{
    pub uart_tx: mpsc::Sender<char>,
}
impl UartNs16550a {
    pub fn new(uart_tx: mpsc::Sender<char>) -> Self {
        Self { uart_tx }
    }
}
impl Device for UartNs16550a {
    fn read(&mut self, _size: u8, _addr: Addr) -> u32 {
        // Default value for always ready. 
        // TODO: implement proper UART behavior
        0x60
    }

    fn write(&mut self, size: u8, addr: Addr, value: u32) -> Result<(), ()> {
        if addr == 0 && size == 1 {
            // print!("{}", (value as u8) as char);
            let _ = self.uart_tx.send(value as u8 as char);
            Ok(())
        } else {
            Err(())
        }
    }
}
