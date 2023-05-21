pub struct ControlRegister {
    name_table1: bool,
    name_table2: bool,
    vram_increment: bool,
    sprite_pattern_addr: bool,
    background_pattern_addr: bool,
    sprite_size: bool,
    master_slave: bool,
    nmi: bool,
}

impl ControlRegister {
    pub fn new() -> Self {
        ControlRegister {
            name_table1: false,
            name_table2: false,
            vram_increment: false,
            sprite_pattern_addr: false,
            background_pattern_addr: false,
            sprite_size: false,
            master_slave: false,
            nmi: false,
        }
    }

    fn get_val(&self) -> u8 {
        let mut val = 0;
        if self.name_table1 {
            val |= 0b1;
        }
        if self.name_table2 {
            val |= 0b10;
        }
        if self.vram_increment {
            val |= 0b100;
        }
        if self.sprite_pattern_addr {
            val |= 0b1000;
        }
        if self.background_pattern_addr {
            val |= 0b10000;
        }
        if self.sprite_size {
            val |= 0b100000;
        }
        if self.master_slave {
            val |= 0b1000000;
        }
        if self.nmi {
            val |= 0b10000000;
        }
        val
    }

    pub fn nametable_addr(&self) -> u16 {
        match self.get_val() & 0b11 {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            3 => 0x2C00,
            _ => panic!("Invalid nametable addr"),
        }
    }

    pub fn vram_increment(&self) -> u8 {
        if self.vram_increment {
            32
        } else {
            1
        }
    }

    pub fn sprt_patter_addr(&self) -> u16 {
        if self.sprite_pattern_addr {
            0x1000
        } else {
            0x0
        }
    }

    pub fn bknd_pattern_addr(&self) -> u16 {
        if self.background_pattern_addr {
            0x1000
        } else {
            0x0
        }
    }

    pub fn generate_vblank_nmi(&self) -> bool {
        self.nmi
    }

    pub fn update(&mut self, data: u8) {
        self.name_table1 = data & 0b1 != 0;
        self.name_table2 = data & 0b10 != 0;
        self.vram_increment = data & 0b100 != 0;
        self.sprite_pattern_addr = data & 0b1000 != 0;
        self.background_pattern_addr = data & 0b10000 != 0;
        self.sprite_size = data & 0b100000 != 0;
        self.master_slave = data & 0b1000000 != 0;
        self.nmi = data & 0b10000000 != 0;
    }
}
