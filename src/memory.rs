use super::cartridge::Cartridge;

pub struct Memory {
    memory: [u8; 2048],
    cartridge: Cartridge,
}

const RAM: u16 = 0x0000;
const RAM_MIRRORS_END: u16 = 0x1FFF;
const PPU_REGISTERS: u16 = 0x2000;
const PPU_REGISTERS_MIRRORS_END: u16 = 0x3FFF;

impl Memory {
    pub fn new(cartridge: Cartridge) -> Self {
        return Memory {
            memory: [0; 2048],
            cartridge,
        };
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            RAM ..= RAM_MIRRORS_END => {
                let mirror_down_addr = addr & 0b00000111_11111111;
                self.memory[mirror_down_addr as usize]
            },
            PPU_REGISTERS ..= PPU_REGISTERS_MIRRORS_END => {
                let _mirror_down_addr = addr & 0b00100000_00000111;
                todo!("Not supported yet");
            },
            0x8000 ..= 0xFFFF => {
                let mut addr = addr;
                addr -= 0x8000;
                if self.cartridge.prg_rom.len() == 0x4000 && addr >= 0x4000 {
                    addr %= 0x4000;
                }
                self.cartridge.prg_rom[addr as usize]
            },
            _ => {
                println!("Ignoring memory access {}", addr);
                0
            }
        }
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        match addr {
            RAM ..= RAM_MIRRORS_END => {
                let mirror_down_addr = addr & 0b00000111_11111111;
                self.memory[mirror_down_addr as usize] = data
            },
            PPU_REGISTERS ..= PPU_REGISTERS_MIRRORS_END => {
                let _mirror_down_addr = addr & 0b00100000_00000111;
                todo!("Not supported yet");
            },
            0x8000 ..= 0xFFFF => {
                panic!("Cannot write to Cartridge")
            },
            _ => {
                println!("Ignoring memory access {}", addr);
            }
        }
    }

    pub fn read_u16(&self, addr: u16) -> u16 {
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
}
