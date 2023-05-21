pub struct AddrRegister {
    high_byte: u8,
    low_byte: u8,
    hi_ptr: bool,
}

impl AddrRegister {
    pub fn new() -> Self {
        AddrRegister {
            high_byte: 0,
            low_byte: 0,
            hi_ptr: true,
        }
    }

    fn set(&mut self, data: u16) {
        self.high_byte = (data >> 8) as u8;
        self.low_byte = data as u8;
    }

    pub fn update(&mut self, data: u8) {
        if self.hi_ptr {
            self.high_byte = data;
        } else {
            self.low_byte = data;
        }

        if self.get() > 0x3FFF {
            self.set(self.get() & 0b11111111111111);
        }

        self.hi_ptr = !self.hi_ptr;
    }

    pub fn increment(&mut self, inc: u8) {
        let lo = self.low_byte;
        self.low_byte = self.low_byte.wrapping_add(inc);
        if lo > self.low_byte {
            self.high_byte = self.high_byte.wrapping_add(1);
        }
        if self.get() > 0x3FFF {
            self.set(self.get() & 0b11111111111111);
        }
    }

    pub fn reset_latch(&mut self) {
        self.hi_ptr = true;
    }

    pub fn get(&self) -> u16 {
        ((self.high_byte as u16) << 8) | (self.low_byte as u16)
    }
}
