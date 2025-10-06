pub type Addr = u32;

pub trait Device: Send {
    /// `size` must be 1, 2, or 4. Address is device-local (offset) in this impl.
    fn read(&mut self, size: u8, addr: Addr) -> u32;
    fn write(&mut self, size: u8, addr: Addr, value: u32) -> Result<(), ()>;
}

struct Region {
    base: Addr, // Base address. Everything is offset from this.
    size: u32,  // in bytes
    device: Box<dyn Device>,
}

pub struct Bus {
    regions: Vec<Region>,
}

impl Bus{
    pub fn new() -> Self {
        Self { regions: Vec::new() }
    }

    pub fn add_region(&mut self, base: Addr, size: u32, device: Box<dyn Device>) {
        self.regions.push(Region { base, size, device });
    }

    fn find_region(&mut self, addr: Addr) -> Option<&mut Region> {
        for region in &mut self.regions {
            if addr >= region.base && addr < region.base + region.size {
                return Some(region);
            }
        }
        None
    }

    pub fn read(&mut self, size: u8, addr: Addr) -> Result<u32, ()> {
        if let Some(region) = self.find_region(addr) {
            let offset = addr - region.base;
            Ok(region.device.read(size, offset))
        } else {
            Err(())
        }
    }

    pub fn write(&mut self, size: u8, addr: Addr, value: u32) -> Result<(), ()> {
        if let Some(region) = self.find_region(addr) {
            let offset = addr - region.base;
            region.device.write(size, offset, value)
        } else {
            Err(())
        }
    }
}