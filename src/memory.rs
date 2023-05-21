use crate::{ppu::Ppu, joypad::Joypad};

use super::cartridge::Cartridge;

pub struct Memory<'call> {
    pub memory: [u8; 2048],
    cartridge: Cartridge,
    ppu: Ppu,
    joypad: Joypad,
    cycles: usize,
    gameloop_callback : Box<dyn FnMut(&Ppu, &mut Joypad) + 'call>,
}

const RAM: u16 = 0x0000;
const RAM_MIRRORS_END: u16 = 0x1FFF;
const PPU_REGISTERS: u16 = 0x2000;
const PPU_REGISTERS_MIRRORS_END: u16 = 0x3FFF;

impl<'a> Memory<'a> {
    pub fn new<'call, F>(cartridge: Cartridge, gameloop_callback: F) -> Memory<'call>
    where
        F: FnMut(&Ppu, &mut Joypad) + 'call,
    {
        let chr_rom = cartridge.chr_rom.clone();
        let mirroring = cartridge.mirroring.clone();
        return Memory {
            memory: [0; 2048],
            cartridge,
            ppu: Ppu::new(chr_rom, mirroring),
            joypad: Joypad::new(),
            cycles: 0,
            gameloop_callback: Box::new(gameloop_callback),
        };
    }

    pub fn tick(&mut self, cycles: u8) {
        self.cycles += cycles as usize;
        let nmi_bef = self.ppu.get_nmi().is_some();
        self.ppu.tick(cycles * 3);
        let nmi_after = self.ppu.get_nmi().is_some();

        if !nmi_bef && nmi_after {
            (self.gameloop_callback)(&self.ppu, &mut self.joypad);
        }
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            RAM..=RAM_MIRRORS_END => {
                let mirror_down_addr = addr & 0b00000111_11111111;
                self.memory[mirror_down_addr as usize]
            }
            0x2000 | 0x2001 | 0x2003 | 0x2005 | 0x2006 | 0x4014 => {
                // panic!("Attempted to read from write only PPU register {:X}", addr)
                0
            }
            0x2002 => self.ppu.read_status(),
            0x2004 => self.ppu.read_oam_data(),
            0x2007 => self.ppu.read_data(),
            0x4010 => 0,
            0x4016 => self.joypad.read(),
            0x4017 => 0,
            0x2008..=PPU_REGISTERS_MIRRORS_END => {
                let _mirror_down_addr = addr & 0b00100000_00000111;
                self.read(_mirror_down_addr)
            }
            0x8000..=0xFFFF => {
                let mut addr = addr;
                addr -= 0x8000;
                if self.cartridge.prg_rom.len() == 0x4000 && addr >= 0x4000 {
                    addr %= 0x4000;
                }
                self.cartridge.prg_rom[addr as usize]
            }
            _ => {
                println!("Ignoring memory access {:X}", addr);
                0
            }
        }
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        match addr {
            RAM..=RAM_MIRRORS_END => {
                let mirror_down_addr = addr & 0b00000111_11111111;
                self.memory[mirror_down_addr as usize] = data
            }
            0x2000 => self.ppu.write_to_ctrl(data),
            0x2001 => self.ppu.write_to_mask(data),
            0x2002 => panic!("Attempted to write to read only PPU register 0x2002"),
            0x2003 => self.ppu.write_to_oam_addr(data),
            0x2004 => self.ppu.write_to_oam_data(data),
            0x2005 => self.ppu.write_to_scroll(data),
            0x2006 => self.ppu.write_to_ppu_addr(data),
            0x2007 => self.ppu.write_to_data(data),
            0x4014 => {},
            0x4016 => self.joypad.write(data),
            0x4017 => {},
            0x2008..=PPU_REGISTERS_MIRRORS_END => {
                let _mirror_down_addr = addr & 0b00100000_00000111;
                self.write(_mirror_down_addr, data)
            }
            0x8000..=0xFFFF => {
                panic!("Cannot write to Cartridge {:X}", addr)
            }
            _ => {
                println!("Ignoring memory access {:X}", addr);
            }
        }
    }

    pub fn read_u16(&mut self, addr: u16) -> u16 {
        let lo = self.read(addr) as u16;
        let hi = self.read(addr + 1) as u16;
        (hi << 8) | (lo as u16)
    }

    pub fn write_u16(&mut self, addr: u16, data: u16) {
        self.write(addr, data as u8);
        self.write(addr + 1, (data >> 8) as u8);
    }

    pub fn mass_write(&mut self, start_addr: u16, bytes: &Vec<u8>) {
        self.memory[start_addr as usize..(start_addr as usize + bytes.len())]
            .copy_from_slice(&bytes[..])
    }

    pub fn poll_nmi(&mut self) -> Option<u8> {
        self.ppu.poll_nmi()
    }
}
