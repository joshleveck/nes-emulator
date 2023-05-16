#[cfg(test)]
use crate::cartridge::test::test_cartridge;
use crate::cpu::{AddrModes, Cpu};
use crate::opcodes::OPCODES_MAP;

pub fn trace(cpu: &mut Cpu) -> String {
    let org_pc = cpu.pc;

    let cmd = cpu.memory.read(cpu.pc);
    cpu.pc += 1;
    let opcode = OPCODES_MAP.get(&cmd).unwrap();

    let mut hex = vec![];
    hex.push(cmd);

    let (addr, stored_val) = if opcode.mode == AddrModes::Immd || opcode.mode == AddrModes::NoneAddr
    {
        (0, 0)
    } else {
        let addr = cpu.get_operand_address(&opcode.mode);
        let val = cpu.memory.read(addr);
        (addr, val)
    };

    // Get operand addr will change pc and that is bad
    cpu.pc = org_pc;

    let tmp = match opcode.len {
        1 => match opcode.code {
            0x0A | 0x4A | 0x2A | 0x6A => format!("A "),
            _ => String::from(""),
        },
        2 => {
            let val = cpu.memory.read(cpu.pc + 1);
            hex.push(val);

            match opcode.mode {
                AddrModes::Immd => format!("#${:02X}", val),
                AddrModes::ZeroP => format!("${:02X} = {:02X}", addr, stored_val),
                AddrModes::ZeroPX => format!("${:02X},X @ {:02X} = {:02X}", val, addr, stored_val),
                AddrModes::ZeroPY => format!("${:02X},Y @ {:02X} = {:02X}", val, addr, stored_val),
                AddrModes::IndrX => format!(
                    "(${:02X},X) @ {:02X} = {:04X} = {:02X}",
                    val,
                    (val.wrapping_add(cpu.x)),
                    addr,
                    stored_val
                ),
                AddrModes::IndrY => format!(
                    "(${:02X}),Y = {:04X} @ {:04X} = {:02X}",
                    val,
                    (addr.wrapping_sub(cpu.y as u16)),
                    addr,
                    stored_val
                ),
                AddrModes::NoneAddr => {
                    // assuming local jumps: BNE, BVS, etc....
                    let address: usize = (org_pc as usize + 2).wrapping_add((val as i8) as usize);
                    format!("${:04X}", address)
                }

                _ => panic!(
                    "Unexpected addressing mode {:?} has ops-len 2. code {:02X}",
                    opcode.mode, opcode.code
                ),
            }
        }
        3 => {
            let lo = cpu.memory.read(cpu.pc + 1);
            let hi = cpu.memory.read(cpu.pc + 2);
            hex.push(lo);
            hex.push(hi);

            let val = cpu.memory.read_u16(cpu.pc + 1);

            match opcode.mode {
                AddrModes::Indr => {
                    //jmp indirect
                    let jmp_addr = if val & 0x00FF == 0x00FF {
                        let lo = cpu.memory.read(val);
                        let hi = cpu.memory.read(val & 0xFF00);
                        (hi as u16) << 8 | (lo as u16)
                    } else {
                        cpu.memory.read_u16(val)
                    };

                    // let jmp_addr = cpu.mem_read_u16(address);
                    format!("(${:04X}) = {:04X}", val, jmp_addr)
                }
                AddrModes::Immd => format!("${:04X}", val),
                //AddrModes::Abs => format!("${:04X} ", addr),
                AddrModes::Abs => {
                    if cmd == 0x20 {
                        format!("${:04X} ", addr)
                    } else {
                        format!("${:04X} = {:02X}", addr, stored_val)
                    }
                }
                AddrModes::AbsX => format!("${:04X},X @ {:04X} = {:02X}", val, addr, stored_val),
                AddrModes::AbsY => format!("${:04X},Y @ {:04X} = {:02X}", val, addr, stored_val),
                _ => panic!(
                    "unexpected addressing mode {:?} has ops-len 3. code {:02X}",
                    opcode.mode, opcode.code
                ),
            }
        }
        _ => String::from(""),
    };

    let hex_str = hex
        .iter()
        .map(|z| format!("{:02X}", z))
        .collect::<Vec<String>>()
        .join(" ");
    let asm_str = format!(
        "{:04X}  {:8} {: >4} {}",
        org_pc, hex_str, opcode.mnemonic, tmp
    )
    .trim()
    .to_string();

    format!(
        "{:47} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X}",
        asm_str,
        cpu.a,
        cpu.x,
        cpu.y,
        cpu.p.to_data(),
        cpu.sp
    )
}

#[test]
fn test_format_trace() {
    let mut cpu = Cpu::new(test_cartridge());
    cpu.memory.write(100, 0xa2);
    cpu.memory.write(101, 0x01);
    cpu.memory.write(102, 0xca);
    cpu.memory.write(103, 0x88);
    cpu.memory.write(104, 0x00);

    cpu.pc = 0x64;
    cpu.a = 1;
    cpu.x = 2;
    cpu.y = 3;

    let mut result: Vec<String> = vec![];
    cpu.run_with_callback(|cpu| {
        result.push(trace(cpu));
    });
    assert_eq!(
        "0064  A2 01     LDX #$01                        A:01 X:02 Y:03 P:24 SP:FD",
        result[0]
    );
    assert_eq!(
        "0066  CA        DEX                             A:01 X:01 Y:03 P:24 SP:FD",
        result[1]
    );
    assert_eq!(
        "0067  88        DEY                             A:01 X:00 Y:03 P:26 SP:FD",
        result[2]
    );
}

#[test]
fn test_format_mem_access() {
    let mut cpu = Cpu::new(test_cartridge());

    // ORA ($33), Y
    cpu.memory.write(100, 0x11);
    cpu.memory.write(101, 0x33);

    //data
    cpu.memory.write(0x33, 00);
    cpu.memory.write(0x34, 04);

    //target cell
    cpu.memory.write(0x400, 0xAA);

    cpu.pc = 0x64;
    cpu.y = 0;

    let mut result: Vec<String> = vec![];
    cpu.run_with_callback(|cpu| {
        result.push(trace(cpu));
    });

    assert_eq!(
        "0064  11 33     ORA ($33),Y = 0400 @ 0400 = AA  A:00 X:00 Y:00 P:24 SP:FD",
        result[0]
    );
}

// #[test]
// fn test_0xa9_lda_immd_load_data() {
//     let mut cpu = Cpu::new(test_cartridge());
//     cpu.load(&vec![0xa9, 0x05, 0x00]);
//     cpu.run();

//     assert_eq!(cpu.a, 5);
//     assert!(!cpu.p.zero);
//     assert!(!cpu.p.neg);
// }

// #[test]
// fn test_0xaa_tax_move_a_to_x() {
//     let mut cpu = Cpu::new(test_cartridge());
//     cpu.a = 10;
//     cpu.load(&vec![0xaa, 0x00]);
//     cpu.run();

//     assert_eq!(cpu.x, 10)
// }

// #[test]
// fn test_5_ops_working_together() {
//     let mut cpu = Cpu::new(test_cartridge());
//     cpu.load(&vec![0xa9, 0xc0, 0xaa, 0xe8, 0x00]);
//     cpu.run();

//     assert_eq!(cpu.x, 0xc1)
// }

// #[test]
// fn test_inx_overflow() {
//     let mut cpu = Cpu::new(test_cartridge());
//     cpu.x = 0xff;
//     cpu.load(&vec![0xe8, 0xe8, 0x00]);
//     cpu.run();

//     assert_eq!(cpu.x, 1)
// }

// #[test]
// fn test_lda_from_memory() {
//     let mut cpu = Cpu::new(test_cartridge());
//     cpu.memory.write(0x10, 0x55);

//     cpu.load(&vec![0xa5, 0x10, 0x00]);
//     cpu.run();

//     assert_eq!(cpu.a, 0x55);
// }
