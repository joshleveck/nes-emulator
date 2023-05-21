use self::addr_register::AddrRegister;
use self::control_register::ControlRegister;
use self::mask_register::MaskRegister;
use self::scroll_register::ScrollRegister;
use self::status_register::StatusRegister;
use crate::cartridge::Mirroring;

mod addr_register;
mod control_register;
mod mask_register;
mod scroll_register;
mod status_register;

pub struct Ppu {
    chr_rom: Vec<u8>,
    vram: [u8; 2048],
    mirroring: Mirroring,
    addr: AddrRegister,
    ctrl: ControlRegister,
    mask: MaskRegister,
    status: StatusRegister,
    scroll: ScrollRegister,

    oam_addr: u8,
    oam_data: [u8; 256],
    palette_table: [u8; 32],

    internal_data_buffer: u8,

    cycles: usize,
    scanline: u16,

    nmi_interrupt: Option<u8>,
}

impl Ppu {
    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        Ppu {
            chr_rom,
            vram: [0; 2048],
            mirroring,
            addr: AddrRegister::new(),
            ctrl: ControlRegister::new(),
            mask: MaskRegister::new(),
            status: StatusRegister::new(),
            scroll: ScrollRegister::new(),

            oam_addr: 0,
            oam_data: [0; 256],
            palette_table: [0; 32],
            internal_data_buffer: 0,

            cycles: 0,
            scanline: 0,

            nmi_interrupt: None,
        }
    }

    pub fn tick(&mut self, cycles: u8) -> bool {
        // println!("TICK {}", cycles);
        self.cycles += cycles as usize;
        if self.cycles >= 341 {
            if self.is_sprite_0_hit(self.cycles) {
                self.status.sprite_zero_hit = true;
            }

            self.cycles -= 341;
            self.scanline += 1;

            if self.scanline == 241 {
                self.status.vblank = true;
                self.status.sprite_zero_hit = false;
                if self.ctrl.generate_vblank_nmi() {
                    self.nmi_interrupt = Some(1);
                }
            }

            if self.scanline >= 262 {
                self.scanline = 0;
                self.nmi_interrupt = None;
                self.status.sprite_zero_hit = false;
                self.status.vblank = false;
                return true;
            }
        }

        return false;
    }

    fn is_sprite_0_hit(&self, cycle: usize) -> bool {
        let y = self.oam_data[0] as usize;
        let x = self.oam_data[3] as usize;
        (y == self.scanline as usize) && x <= cycle && self.mask.show_sprites
    }

    pub fn poll_nmi(&mut self) -> Option<u8> {
        let nmi = self.nmi_interrupt;
        self.nmi_interrupt = None;
        nmi
    }

    pub fn write_to_ppu_addr(&mut self, value: u8) {
        self.addr.update(value);
    }

    pub fn write_to_ctrl(&mut self, value: u8) {
        let bef_nmi_status = self.ctrl.generate_vblank_nmi();
        self.ctrl.update(value);

        if !bef_nmi_status && self.ctrl.generate_vblank_nmi() && self.status.vblank {
            self.nmi_interrupt = Some(1);
        }
    }

    pub fn write_to_mask(&mut self, value: u8) {
        self.mask.update(value)
    }

    pub fn write_to_scroll(&mut self, value: u8) {
        self.scroll.write(value);
    }

    pub fn read_status(&mut self) -> u8 {
        let data = self.status.get_val();
        self.status.vblank = false;
        self.addr.reset_latch();
        self.scroll.reset_latch();
        data
    }

    pub fn write_to_oam_addr(&mut self, value: u8) {
        self.oam_addr = value;
    }

    pub fn write_to_oam_data(&mut self, value: u8) {
        self.oam_data[self.oam_addr as usize] = value;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    pub fn write_oam_dma(&mut self, data: &[u8; 256]) {
        for x in data.iter() {
            self.oam_data[self.oam_addr as usize] = *x;
            self.oam_addr = self.oam_addr.wrapping_add(1);
        }
    }

    pub fn read_oam_data(&self) -> u8 {
        self.oam_data[self.oam_addr as usize]
    }

    pub fn ctrl_bknd(&self) -> u16 {
        self.ctrl.bknd_pattern_addr()
    }

    pub fn ctrl_sptr(&self) -> u16 {
        self.ctrl.sprt_patter_addr()
    }

    pub fn get_vram(&self, addr: usize) -> u8 {
        self.vram[addr as usize]
    }

    pub fn get_chr_rom(&self, start: u16, end: u16) -> &[u8] {
        &self.chr_rom[start as usize..=end as usize]
    }

    pub fn get_nmi(&self) -> Option<u8> {
        self.nmi_interrupt
    }

    pub fn get_palette_table(&self, addr: usize) -> u8 {
        self.palette_table[addr]
    }

    pub fn get_oam_data(&self) -> &[u8] {
        &self.oam_data
    }

    pub fn get_mirroring(&self) -> &Mirroring {
        &self.mirroring
    }

    pub fn get_scroll(&self) -> &ScrollRegister {
        &self.scroll
    }

    pub fn get_vram_span(&self, start: u16, end: u16) -> &[u8] {
        &self.vram[start as usize..end as usize]
    }

    pub fn get_ctrl_nametable(&self) -> u16 {
        self.ctrl.nametable_addr()
    }

    fn increment_vram_addr(&mut self) {
        self.addr.increment(self.ctrl.vram_increment());
    }

    pub fn mirror_vram_addr(&self, addr: u16) -> u16 {
        let mirrored_vram = addr & 0b10111111111111; // mirror down 0x3000-0x3eff to 0x2000 - 0x2eff
        let vram_index = mirrored_vram - 0x2000; // to vram vector
        let name_table = vram_index / 0x400; // to the name table index
        match (&self.mirroring, name_table) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => vram_index - 0x800,
            (Mirroring::Horizontal, 2) => vram_index - 0x400,
            (Mirroring::Horizontal, 1) => vram_index - 0x400,
            (Mirroring::Horizontal, 3) => vram_index - 0x800,
            _ => vram_index,
        }
    }

    pub fn write_to_data(&mut self, value: u8) {
        let addr = self.addr.get();
        match addr {
            0..=0x1FFF => println!("Attempt to write to chr rom {:X}", addr),
            0x2000..=0x2FFF => self.vram[self.mirror_vram_addr(addr) as usize] = value,
            0x3000..=0x3EFF => unimplemented!("Attempt to write to vram mirror {:X}", addr),
            0x3F10 | 0x3F14 | 0x3F18 | 0x3F1C => {
                let addr_mirror = addr - 0x10;
                self.palette_table[(addr_mirror - 0x3F00) as usize] = value;
            }
            0x3F00..=0x3FFF => self.palette_table[(addr - 0x3F00) as usize] = value,
            _ => panic!("Invalid address: {:X}", addr),
        }

        self.increment_vram_addr();
    }

    pub fn read_data(&mut self) -> u8 {
        let addr = self.addr.get();
        self.increment_vram_addr();

        match addr {
            0x0000..=0x1FFF => {
                let res = self.internal_data_buffer;
                self.internal_data_buffer = self.chr_rom[addr as usize];
                res
            }
            0x2000..=0x3EFF => {
                let res = self.internal_data_buffer;
                self.internal_data_buffer = self.vram[self.mirror_vram_addr(addr) as usize];
                res
            }
            0x3F00..=0x3FFF => self.palette_table[(addr - 0x3F00) as usize],
            _ => panic!("Invalid address: {:#X}", addr),
        }
    }
}
