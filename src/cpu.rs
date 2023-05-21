use crate::{cartridge::Cartridge, joypad::Joypad, opcodes::OPCODES_MAP, ppu::Ppu};

use super::memory;

pub mod test;

const STACK_INIT: u8 = 0xfd;
const STACK: u16 = 0x0100;
const STATUS_INIT: u8 = 0x24;

#[derive(Debug, PartialEq)]
pub enum AddrModes {
    Immd,
    ZeroP,
    ZeroPX,
    ZeroPY,
    Rel,
    Abs,
    AbsX,
    AbsY,
    Indr,
    IndrX,
    IndrY,
    NoneAddr,
}

pub struct Status {
    carry: bool,
    zero: bool,
    intr_d: bool,
    dec: bool,
    brk: bool,
    brk2: bool,
    ovflw: bool,
    neg: bool,
}

impl Status {
    fn new() -> Self {
        Status {
            carry: false,
            zero: false,
            intr_d: true,
            dec: false,
            brk: false,
            brk2: true,
            ovflw: false,
            neg: false,
        }
    }

    fn reset(&mut self) {
        self.from_data(STATUS_INIT);
    }

    fn to_data(&self) -> u8 {
        let mut res: u8 = 0;
        if self.neg {
            res |= 1 << 7;
        }
        if self.ovflw {
            res |= 1 << 6;
        }
        if self.brk2 {
            res |= 1 << 5;
        }
        if self.brk {
            res |= 1 << 4;
        }
        if self.dec {
            res |= 1 << 3;
        }
        if self.intr_d {
            res |= 1 << 2;
        }
        if self.zero {
            res |= 1 << 1;
        }
        if self.carry {
            res |= 1;
        }

        res
    }

    fn from_data(&mut self, data: u8) {
        self.carry = data & 1 != 0;
        self.zero = data & (1 << 1) != 0;
        self.intr_d = data & (1 << 2) != 0;
        self.dec = data & (1 << 3) != 0;
        self.brk = data & (1 << 4) != 0;
        self.brk2 = data & (1 << 5) != 0;
        self.ovflw = data & (1 << 6) != 0;
        self.neg = data & (1 << 7) != 0;
    }
}

pub struct Cpu<'call> {
    a: u8,
    x: u8,
    y: u8,
    pc: u16,
    sp: u8,
    p: Status,
    pub memory: memory::Memory<'call>,
}

impl<'a> Cpu<'a> {
    pub fn new<'call, F>(cartridge: Cartridge, mem_callback: F) -> Cpu<'call>
    where
        F: FnMut(&Ppu, &mut Joypad) + 'call,
    {
        return Cpu {
            a: 0,
            x: 0,
            y: 0,
            pc: 0x8000,
            sp: STACK_INIT,
            p: Status::new(),
            memory: memory::Memory::new(cartridge, mem_callback),
        };
    }

    fn stack_push(&mut self, data: u8) {
        self.memory.write(STACK + self.sp as u16, data);
        self.sp -= 1;
    }

    fn stack_push_u16(&mut self, data: u16) {
        let hi = (data >> 8) as u8;
        let low = (data & 0xff) as u8;

        self.stack_push(hi);
        self.stack_push(low);
    }

    fn stack_pop(&mut self) -> u8 {
        self.sp += 1;
        self.memory.read(STACK + self.sp as u16)
    }

    fn stack_pop_u16(&mut self) -> u16 {
        let low = self.stack_pop() as u16;
        let hi = self.stack_pop() as u16;

        hi << 8 | low
    }

    fn update_z_n(&mut self, result: u8) {
        self.p.zero = result == 0;
        self.p.neg = result & (1 << 7) != 0
    }

    fn page_cross(&self, addr1: u16, addr2: u16) -> bool {
        addr1 & 0xFF00 != addr2 & 0xFF00
    }

    fn get_operand_address(&mut self, mode: &AddrModes) -> (u16, bool) {
        match mode {
            // Encoded in the instruction
            AddrModes::Immd => {
                self.pc += 1;
                (self.pc - 1, false)
            }
            // Zero page is addresses 00-FF (1 byte)
            // Encoded in the instruction
            AddrModes::ZeroP => {
                self.pc += 1;
                (self.memory.read(self.pc - 1) as u16, false)
            }
            AddrModes::ZeroPX => {
                let param = self.memory.read(self.pc);
                self.pc += 1;
                // Specs allow overflow
                let addr = param.wrapping_add(self.x) as u16;
                (addr, false)
            }
            AddrModes::ZeroPY => {
                let param = self.memory.read(self.pc);
                self.pc += 1;
                // Specs allow overflow
                let addr = param.wrapping_add(self.y) as u16;
                (addr, false)
            }
            AddrModes::Rel => {
                let param = self.memory.read(self.pc) as i8;

                (self.pc.wrapping_add(1).wrapping_add(param as u16), false)
            }
            AddrModes::Abs => {
                let param = self.memory.read_u16(self.pc);
                self.pc += 2;
                (param, false)
            }
            AddrModes::AbsX => {
                let param = self.memory.read_u16(self.pc);
                let addr = param.wrapping_add(self.x as u16);
                self.pc += 2;
                (addr, self.page_cross(param, addr))
            }
            AddrModes::AbsY => {
                let param = self.memory.read_u16(self.pc);
                let addr = param.wrapping_add(self.y as u16);
                self.pc += 2;
                (addr, self.page_cross(param, addr))
            }
            AddrModes::Indr => {
                let param = self.memory.read_u16(self.pc);
                self.pc += 2;
                (self.memory.read_u16(param), false)
            }
            AddrModes::IndrX => {
                let base = self.memory.read(self.pc);
                self.pc += 1;

                let ptr: u8 = (base as u8).wrapping_add(self.x);
                let lo = self.memory.read(ptr as u16);
                let hi = self.memory.read(ptr.wrapping_add(1) as u16);
                ((hi as u16) << 8 | (lo as u16), false)
            }
            AddrModes::IndrY => {
                let base = self.memory.read(self.pc);
                self.pc += 1;

                let lo = self.memory.read(base as u16);
                let hi = self.memory.read((base as u8).wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let deref = deref_base.wrapping_add(self.y as u16);

                (deref, self.page_cross(deref_base, deref))
            }
            AddrModes::NoneAddr => panic!("No address mode"),
        }
    }

    // fn cmd_to_addr(&mut self, cmd: u8) -> u16 {
    //     let addr_spec = cmd & !(0b111 << 5);
    //     self.get_operand_address(match addr_spec {
    //         0x9 => &AddrModes::Immd,
    //         0x5 => &AddrModes::ZeroP,
    //         0xD => &AddrModes::Abs,
    //         0x1 => &AddrModes::IndrX,
    //         0x11 => &AddrModes::IndrY,
    //         0x19 => &AddrModes::AbsY,
    //         0x15 => &AddrModes::ZeroPX,
    //         0x1D => &AddrModes::AbsX,
    //         _ => panic!("Unrecognized address mode {}", cmd),
    //     })
    // }

    fn branch(&mut self, cond: bool) {
        if cond {
            self.memory.tick(1);
            self.pc = self.get_operand_address(&AddrModes::Rel).0;
        } else {
            self.pc += 1;
        }
    }

    fn add_with_carry(&mut self, data: u8) {
        let mut result = self.a as u16 + data as u16;
        if self.p.carry {
            result += 1;
        }
        self.p.carry = result > 0xff;
        self.p.ovflw = (self.a ^ result as u8) & (data ^ result as u8) & 0x80 != 0;

        self.a = result as u8;
        self.update_z_n(self.a);
    }

    fn adc(&mut self, addr: u16) {
        let data = self.memory.read(addr);
        self.add_with_carry(data);
    }

    fn and(&mut self, addr: u16) {
        self.a &= self.memory.read(addr);
        self.update_z_n(self.a);
    }

    fn bit(&mut self, addr: u16) {
        let data = self.memory.read(addr);
        self.p.zero = self.a & data == 0;
        self.p.neg = data & (1 << 7) != 0;
        self.p.ovflw = data & (1 << 6) != 0;
    }

    fn cmp(&mut self, reg: u8, addr: u16) {
        let data = self.memory.read(addr);
        self.p.carry = reg >= data;
        self.update_z_n(reg.wrapping_sub(data));
    }

    fn eor(&mut self, addr: u16) {
        let data = self.memory.read(addr);
        self.a ^= data;
        self.update_z_n(self.a);
    }

    fn lda(&mut self, addr: u16) {
        self.a = self.memory.read(addr);
        self.update_z_n(self.a);
    }

    fn ldx(&mut self, addr: u16) {
        self.x = self.memory.read(addr);
        self.update_z_n(self.x);
    }

    fn ldy(&mut self, addr: u16) {
        self.y = self.memory.read(addr);
        self.update_z_n(self.y);
    }

    fn sbc(&mut self, addr: u16) {
        let data = self.memory.read(addr);
        self.add_with_carry(((data as i8).wrapping_neg().wrapping_sub(1)) as u8);
    }

    fn nmi(&mut self) {
        self.stack_push_u16(self.pc);
        let mut flag = Status::new();
        flag.from_data(self.p.to_data());

        flag.brk = false;
        flag.brk2 = true;

        self.stack_push(flag.to_data());
        self.p.intr_d = true;

        self.memory.tick(2);
        self.pc = self.memory.read_u16(0xFFFA);
    }

    pub fn load(&mut self, program: &Vec<u8>) {
        self.memory.mass_write(0x0600, &program);
        self.memory.write_u16(0xFFFC, 0x0600);
    }

    pub fn reset(&mut self) {
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.sp = STACK_INIT;
        self.p.reset();

        self.pc = self.memory.read_u16(0xFFFC);
    }

    pub fn set_pc(&mut self, pc: u16) {
        self.pc = pc;
    }

    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }

    pub fn run_with_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut Cpu),
    {
        loop {
            if let Some(_nmi) = self.memory.poll_nmi() {
                self.nmi();
            }
            callback(self);

            let cmd = self.memory.read(self.pc);
            self.pc += 1;

            let opcode = OPCODES_MAP
                .get(&cmd)
                .expect(&format!("Unrecognized opcode {}", cmd));

            match cmd {
                // ADC
                0x69 | 0x65 | 0x75 | 0x6D | 0x7D | 0x79 | 0x61 | 0x71 => {
                    let (addr, cross_res) = self.get_operand_address(&opcode.mode);
                    if cross_res {
                        self.memory.tick(1);
                    }
                    self.adc(addr);
                }
                // AND
                0x29 | 0x25 | 0x35 | 0x2d | 0x3d | 0x39 | 0x21 | 0x31 => {
                    let (addr, cross_res) = self.get_operand_address(&opcode.mode);
                    if cross_res {
                        self.memory.tick(1);
                    }
                    self.and(addr)
                }
                // ASL
                // Accumulator
                0x0A => {
                    self.p.carry = self.a & 0x80 != 0;
                    self.a = self.a << 1;
                    self.update_z_n(self.a);
                }
                // Memory
                0x06 | 0x16 | 0x0E | 0x1E => {
                    let (addr, _) = self.get_operand_address(&opcode.mode);
                    let mut data = self.memory.read(addr);
                    self.p.carry = data & 0x80 != 0;
                    data = data << 1;
                    self.update_z_n(data);
                    self.memory.write(addr, data);
                }
                // BCC
                0x90 => self.branch(!self.p.carry),
                // BCS
                0xB0 => self.branch(self.p.carry),
                // BEQ
                0xF0 => self.branch(self.p.zero),
                // BIT
                0x24 | 0x2C => {
                    let (addr, _) = self.get_operand_address(&opcode.mode);

                    self.bit(addr);
                }
                // BMI
                0x30 => self.branch(self.p.neg),
                // BNE
                0xD0 => self.branch(!self.p.zero),
                // BPL
                0x10 => self.branch(!self.p.neg),
                // BRK
                0x00 => return,
                // BVC
                0x50 => self.branch(!self.p.ovflw),
                // BVS
                0x70 => self.branch(self.p.ovflw),
                // CLC
                0x18 => self.p.carry = false,
                // CLD
                0xD8 => self.p.dec = false,
                // CLI
                0x58 => self.p.intr_d = false,
                // CLV
                0xB8 => self.p.ovflw = false,
                // CMP
                0xC9 | 0xC5 | 0xD5 | 0xCD | 0xDD | 0xD9 | 0xC1 | 0xD1 => {
                    let (addr, cross_res) = self.get_operand_address(&opcode.mode);
                    if cross_res {
                        self.memory.tick(1);
                    }
                    self.cmp(self.a, addr);
                }
                // CPX
                0xE0 | 0xE4 | 0xEC => {
                    let (addr, _) = self.get_operand_address(&opcode.mode);
                    self.cmp(self.x, addr);
                }
                // CPY
                0xC0 | 0xC4 | 0xCC => {
                    let (addr, _) = self.get_operand_address(&opcode.mode);
                    self.cmp(self.y, addr);
                }
                // DCP
                0xC7 | 0xD7 | 0xCF | 0xDF | 0xDB | 0xD3 | 0xC3 => {
                    let (addr, _) = self.get_operand_address(&opcode.mode);
                    let mut data = self.memory.read(addr);
                    data = data.wrapping_sub(1);
                    self.memory.write(addr, data);
                    if data <= self.a {
                        self.p.carry = true;
                    }

                    self.update_z_n(self.a.wrapping_sub(data));
                }
                // DEC
                0xC6 | 0xD6 | 0xCE | 0xDE => {
                    let (addr, _) = self.get_operand_address(&opcode.mode);
                    let mut data = self.memory.read(addr);
                    data = data.wrapping_sub(1);
                    self.update_z_n(data);
                    self.memory.write(addr, data);
                }
                // DEX
                0xCA => {
                    self.x = self.x.wrapping_sub(1);
                    self.update_z_n(self.x);
                }
                // DEY
                0x88 => {
                    self.y = self.y.wrapping_sub(1);
                    self.update_z_n(self.y)
                }
                // EOR
                0x49 | 0x45 | 0x55 | 0x4D | 0x5D | 0x59 | 0x41 | 0x51 => {
                    let (addr, cross_res) = self.get_operand_address(&opcode.mode);
                    if cross_res {
                        self.memory.tick(1);
                    }
                    self.eor(addr);
                }
                // INC
                0xE6 | 0xF6 | 0xEE | 0xFE => {
                    let (addr, _) = self.get_operand_address(&opcode.mode);
                    let mut data = self.memory.read(addr);
                    data = data.wrapping_add(1);
                    self.update_z_n(data);
                    self.memory.write(addr, data);
                }
                // INX
                0xE8 => {
                    self.x = self.x.wrapping_add(1);
                    self.update_z_n(self.x)
                }
                // INY
                0xC8 => {
                    self.y = self.y.wrapping_add(1);
                    self.update_z_n(self.y)
                }
                // ISB
                0xE7 | 0xF7 | 0xEF | 0xFF | 0xFB | 0xE3 | 0xF3 => {
                    let (addr, _) = self.get_operand_address(&opcode.mode);
                    let mut data = self.memory.read(addr);
                    data = data.wrapping_add(1);
                    self.update_z_n(data);
                    self.memory.write(addr, data);
                    self.add_with_carry(((data as i8).wrapping_neg().wrapping_sub(1)) as u8);
                }
                // JMP absolute
                0x4C => {
                    let (addr, _) = self.get_operand_address(&opcode.mode);
                    self.pc = self.memory.read_u16(addr);
                }
                // JMP indirect
                0x6C => {
                    let mem_address = self.memory.read_u16(self.pc);

                    let indirect_ref = if mem_address & 0x00FF == 0x00FF {
                        let lo = self.memory.read(mem_address);
                        let hi = self.memory.read(mem_address & 0xFF00);
                        (hi as u16) << 8 | (lo as u16)
                    } else {
                        self.memory.read_u16(mem_address)
                    };

                    self.pc = indirect_ref;
                }
                // JSR
                0x20 => {
                    self.stack_push_u16(self.pc + 1);
                    let addr = self.memory.read_u16(self.pc);
                    self.pc = addr;
                }
                // LAX
                0xA7 | 0xB7 | 0xAF | 0xBF | 0xA3 | 0xB3 => {
                    let (addr, _) = self.get_operand_address(&opcode.mode);
                    let data = self.memory.read(addr);
                    self.a = data;
                    self.update_z_n(self.a);
                    self.x = self.a;
                }
                // LDA
                0xA9 | 0xA5 | 0xB5 | 0xAD | 0xBD | 0xB9 | 0xA1 | 0xB1 => {
                    let (addr, cross_res) = self.get_operand_address(&opcode.mode);
                    if cross_res {
                        self.memory.tick(1);
                    }
                    self.lda(addr);
                }
                // LDX
                0xA2 | 0xA6 | 0xB6 | 0xAE | 0xBE => {
                    let (addr, cross_res) = self.get_operand_address(&opcode.mode);
                    if cross_res {
                        self.memory.tick(1);
                    }
                    self.ldx(addr);
                }
                // LDY
                0xA0 | 0xA4 | 0xB4 | 0xAC | 0xBC => {
                    let (addr, cross_res) = self.get_operand_address(&opcode.mode);
                    if cross_res {
                        self.memory.tick(1);
                    }
                    self.ldy(addr);
                }
                // LSR
                // Accumulator
                0x4A => {
                    self.p.carry = self.a & 1 == 1;
                    self.a >>= 1;
                    self.update_z_n(self.a);
                }
                // Memory
                0x46 | 0x56 | 0x4E | 0x5E => {
                    let (addr, _) = self.get_operand_address(&opcode.mode);
                    let mut data = self.memory.read(addr);
                    self.p.carry = data & 1 == 1;
                    data >>= 1;
                    self.memory.write(addr, data);
                    self.update_z_n(data);
                }
                // NOP
                0xEA | 0x02 | 0x12 | 0x22 | 0x32 | 0x42 | 0x52 | 0x62 | 0x72 | 0x92 | 0xB2
                | 0xD2 | 0xF2 | 0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => {}
                // NOP read
                0x04 | 0x44 | 0x64 | 0x14 | 0x34 | 0x54 | 0x74 | 0xD4 | 0xF4 | 0x0C | 0x1C
                | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => {
                    let (addr, _) = self.get_operand_address(&opcode.mode);
                    let _data = self.memory.read(addr);
                }
                // ORA
                0x09 | 0x05 | 0x15 | 0x0D | 0x1D | 0x19 | 0x01 | 0x11 => {
                    let (addr, cross_res) = self.get_operand_address(&opcode.mode);
                    if cross_res {
                        self.memory.tick(1);
                    }
                    let data = self.memory.read(addr);
                    self.a |= data;
                    self.update_z_n(self.a);
                }
                // PHA
                0x48 => self.stack_push(self.a),
                // PHP
                0x08 => {
                    let mut status_cpy = Status::new();
                    status_cpy.from_data(self.p.to_data());
                    status_cpy.brk = true;
                    status_cpy.brk2 = true;
                    let data = status_cpy.to_data();
                    self.stack_push(data);
                }
                // PLA
                0x68 => {
                    self.a = self.stack_pop();
                    self.update_z_n(self.a)
                }
                // PLP
                0x28 => {
                    let data = self.stack_pop();
                    self.p.from_data(data);
                    self.p.brk = false;
                    self.p.brk2 = true;
                }
                // RLA
                0x27 | 0x37 | 0x2F | 0x3F | 0x3B | 0x33 | 0x23 => {
                    let (addr, _) = self.get_operand_address(&opcode.mode);
                    let mut data = self.memory.read(addr);
                    let carry = self.p.carry;
                    self.p.carry = data & (1 << 7) != 0;
                    data <<= 1;
                    if carry {
                        data |= 1;
                    }
                    self.memory.write(addr, data);
                    self.update_z_n(data);
                    self.a &= data;
                }
                // ROL
                // Accumulator
                0x2A => {
                    let carry = self.p.carry;
                    self.p.carry = self.a & (1 << 7) != 0;
                    self.a <<= 1;
                    if carry {
                        self.a |= 1;
                    }
                    self.update_z_n(self.a);
                }
                // Memory
                0x26 | 0x36 | 0x2E | 0x3E => {
                    let (addr, _) = self.get_operand_address(&opcode.mode);
                    let mut data = self.memory.read(addr);
                    let carry = self.p.carry;
                    self.p.carry = data & (1 << 7) != 0;
                    data <<= 1;
                    if carry {
                        data |= 1;
                    }
                    self.memory.write(addr, data);
                    self.update_z_n(data);
                }
                // ROR
                // Accumulator
                0x6A => {
                    let carry = self.p.carry;
                    self.p.carry = self.a & 1 == 1;
                    self.a >>= 1;
                    if carry {
                        self.a |= 1 << 7;
                    }
                    self.update_z_n(self.a)
                }
                // Memory
                0x66 | 0x76 | 0x6E | 0x7E => {
                    let (addr, _) = self.get_operand_address(&opcode.mode);
                    let mut data = self.memory.read(addr);
                    let carry = self.p.carry;
                    self.p.carry = data & 1 == 1;
                    data >>= 1;
                    if carry {
                        data |= 1 << 7;
                    }
                    self.memory.write(addr, data);
                    self.update_z_n(data);
                }
                // RRA
                0x67 | 0x77 | 0x6F | 0x7F | 0x7B | 0x63 | 0x73 => {
                    let (addr, _) = self.get_operand_address(&opcode.mode);
                    let mut data = self.memory.read(addr);
                    let carry = self.p.carry;
                    self.p.carry = data & 1 == 1;
                    data >>= 1;
                    if carry {
                        data |= 1 << 7;
                    }
                    self.memory.write(addr, data);
                    self.update_z_n(data);
                    self.add_with_carry(data);
                }
                // RTI
                0x40 => {
                    let data = self.stack_pop();
                    self.p.from_data(data);
                    self.p.brk = false;
                    self.p.brk2 = true;
                    self.pc = self.stack_pop_u16();
                }
                // RTS
                0x60 => {
                    self.pc = self.stack_pop_u16();
                    self.pc += 1;
                }
                // SAX
                0x87 | 0x97 | 0x8F | 0x83 => {
                    let data = self.a & self.x;
                    let (addr, _) = self.get_operand_address(&opcode.mode);
                    self.memory.write(addr, data);
                }
                // SBC
                // Note: 0xEB is an unofficial opcode
                0xE9 | 0xE5 | 0xF5 | 0xED | 0xFD | 0xF9 | 0xE1 | 0xF1 | 0xEB => {
                    let (addr, cross_res) = self.get_operand_address(&opcode.mode);
                    if cross_res {
                        self.memory.tick(1);
                    }
                    self.sbc(addr);
                }
                // SEC
                0x38 => self.p.carry = true,
                // SED
                0xF8 => self.p.dec = true,
                // SEI
                0x78 => self.p.intr_d = true,
                // SKB
                0x80 | 0x82 | 0x89 | 0xc2 | 0xe2 => {
                    let _data = self.get_operand_address(&AddrModes::Immd);
                }
                // SLO
                0x07 | 0x17 | 0x0F | 0x1F | 0x1B | 0x03 | 0x13 => {
                    let (addr, _) = self.get_operand_address(&opcode.mode);
                    let mut data = self.memory.read(addr);
                    self.p.carry = data & 0x80 != 0;
                    data = data << 1;
                    self.update_z_n(data);
                    self.memory.write(addr, data);
                    self.a |= data;
                    self.update_z_n(self.a);
                }
                // SRE
                0x47 | 0x57 | 0x4F | 0x5F | 0x5B | 0x43 | 0x53 => {
                    let (addr, _) = self.get_operand_address(&opcode.mode);
                    let mut data = self.memory.read(addr);
                    self.p.carry = data & 1 == 1;
                    data >>= 1;
                    self.memory.write(addr, data);
                    self.update_z_n(data);
                    self.a ^= data;
                    self.update_z_n(self.a);
                }
                // STA
                0x85 | 0x95 | 0x8D | 0x9D | 0x99 | 0x81 | 0x91 => {
                    let (addr, _) = self.get_operand_address(&opcode.mode);
                    self.memory.write(addr, self.a);
                }
                // STX
                0x86 | 0x96 | 0x8E => {
                    let (addr, _) = self.get_operand_address(&opcode.mode);
                    self.memory.write(addr, self.x);
                }
                // STY
                0x84 | 0x94 | 0x8C => {
                    let (addr, _) = self.get_operand_address(&opcode.mode);
                    self.memory.write(addr, self.y);
                }
                // TAX
                0xAA => {
                    self.x = self.a;
                    self.update_z_n(self.x)
                }
                // TAY
                0xA8 => {
                    self.y = self.a;
                    self.update_z_n(self.y)
                }
                // TSX
                0xBA => {
                    self.x = self.sp;
                    self.update_z_n(self.x)
                }
                // TXA
                0x8A => {
                    self.a = self.x;
                    self.update_z_n(self.a)
                }
                // TXS
                0x9A => self.sp = self.x,
                // TYA
                0x98 => {
                    self.a = self.y;
                    self.update_z_n(self.a)
                }
                _ => panic!(
                    "Unrecognized command {:X} at PC {:X}\nPrevious cmds {:X} {:X}",
                    cmd,
                    self.pc,
                    self.memory.read(self.pc - 1),
                    self.memory.read(self.pc - 2)
                ),
            }

            self.memory.tick(opcode.cycles);
        }
    }

    pub fn insert_cartridge(&mut self, program: &Vec<u8>) {
        self.load(program);
        self.reset();
        self.run();
    }
}
