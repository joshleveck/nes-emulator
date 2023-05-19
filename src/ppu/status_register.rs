pub struct StatusRegister {
    pub sprite_overflow: bool,
    pub sprite_zero_hit: bool,
    pub vblank: bool,
}

impl StatusRegister {
    pub fn new() -> Self {
        StatusRegister {
            sprite_overflow: false,
            sprite_zero_hit: false,
            vblank: false,
        }
    }

    pub fn get_val(&self) -> u8 {
        let mut val = 0;
        if self.sprite_overflow {
            val |= 0b0010_0000;
        }
        if self.sprite_zero_hit {
            val |= 0b0100_0000;
        }
        if self.vblank {
            val |= 0b1000_0000;
        }
        val
    }
}