use crate::bus::{Addr, Device};

pub struct Ram {
    size: u32,
    data: Vec<u8>,
}
impl Ram {
    pub fn new(size: usize) -> Self {
        print!("Initialized {} bytes in ram\n", size);
        Self {
            size: size as u32,
            data: vec![0; size],
        }
    }
    pub fn size(&self) -> u32 { self.size }
}


impl Device for Ram {
    fn read(&mut self, size: u8, addr: Addr) -> u32 {
        let o = addr as usize;
        match size {
            1 => self.data[o] as u32,
            2 => {
                let bytes: [u8; 2] = self.data[o..o + 2]
                    .try_into()
                    .expect("RAM: 16-bit read OOB");
                u16::from_le_bytes(bytes) as u32
            }
            4 => {
                let bytes: [u8; 4] = self.data[o..o + 4]
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
                self.data[o] = value as u8;
            }
            2 => {
                let bytes = (value as u16).to_le_bytes(); // 2 bytes
                self.data[o..o + 2].copy_from_slice(&bytes);
            }
            4 => {
                let bytes = value.to_le_bytes(); // 4 bytes
                self.data[o..o + 4].copy_from_slice(&bytes);
            }
            _ => return Err(()),
        }
        Ok(())
    }
}
