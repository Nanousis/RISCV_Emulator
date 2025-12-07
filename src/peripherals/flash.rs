use std::panic;

use crate::{bus::{Addr, Device}};

pub struct Flash {
    // addr: Addr,
    size: u32,
    data: Vec<u8>,
    addr_reg: u32,
    data_in_reg: u32,
    ren: bool,
    wen: bool,
    // screen_csr: Arc<ScreenCSRShared>,
    // pub screen_tx: mpsc::Sender<ScreenMsg>,

}
impl Flash {
    pub fn new(size: usize, data: Vec<u8>) -> Self {
        println!("Initialized {} bytes in Flash", size);
        let mut mem = vec![0; size];
        let copy_len = data.len().min(size);
        mem[..copy_len].copy_from_slice(&data[..copy_len]);
        Self {
            size: size as u32,
            data: mem,
            addr_reg: 0,
            data_in_reg: 0,
            ren: false,
            wen: false,
        }
    }
    pub fn size(&self) -> u32 { self.size }
}

// #define FLASH_READY 0x8B000000
// #define FLASH_REN 0x8B000001
// #define FLASH_WEN 0x8B000002
// #define FLASH_ADRESS 0x8B000004
// #define FLASH_DATA_IN 0x8B000008
// #define FLASH_DATA_OUT 0x8B00000C
impl Device for Flash {
    fn read(&mut self, size: u8, addr: Addr) -> u32 {
        let o = addr as usize;
        match size {
            1 => {
                if addr as usize == 0 { // ready register always ready.
                    return 0xff;
                }
                else {
                    panic!("Flash: Invalid read from control registers");
                }
            }
            4 => {
                if addr as usize == 4{
                    panic!("Flash: Read from address register not supported");
                }
                else if addr as usize == 8{
                    panic!("Flash: Read from data in register not supported");
                }
                else if addr as usize == 12{
                    let address = self.addr_reg as usize;
                    let bytes: [u8; 4] = self.data[address..address + 4]
                        .try_into()
                        .expect("Flash: 32-bit read OOB");
                    u32::from_le_bytes(bytes)
                }
                else {
                    panic!("Flash: Invalid read from control registers");
                }
            }
            _ => panic!("Invalid read size: {size}"),
        }
    }

    fn write(&mut self, size: u8, addr: Addr, value: u32) -> Result<(), ()> {
        let o = addr as usize;
        match  size{
            1 => {
                match addr as usize {
                    1 => { // REN
                        // Do nothing for read enable
                        self.ren = value != 0;
                    }
                    2 => { // WEN
                        // Do nothing for write enable
                        self.wen = value != 0;
                    }
                    _ => {
                        panic!("Flash: Invalid write to control registers");
                    }
                }
            }
            4 => {
                match addr as usize {
                    4 => { // ADDRESS
                        self.addr_reg = value;
                    }
                    8 => { // DATA IN
                        panic!("Flash: Writing to flash is not yet supported");
                    }
                    12 => { // DATA OUT (trigger write)
                        panic!("Flash: Writing to Data out is not supported");
                    }
                    _ => {
                        panic!("Flash: Invalid write to control registers");
                    }
                }
            }
            _ => return Err(()),
            
        }
        Ok(())
    }
}
