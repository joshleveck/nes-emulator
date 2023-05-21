use super::palette;

pub struct Frame {
    pub data: Vec<u8>,
}


impl Frame {
    const WIDTH: usize = 256;
    const HEIGHT: usize = 240;

    pub fn new() -> Self {
        Frame {
            data: vec![0; Frame::WIDTH * Frame::HEIGHT * 3],
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, rgb: (u8, u8, u8)) {
        let offset = (y * Frame::WIDTH + x) * 3;
        self.data[offset] = rgb.0;
        self.data[offset + 1] = rgb.1;
        self.data[offset + 2] = rgb.2;
    }

    
}

fn show_tile(chr_rom: &Vec<u8>, bank: usize, tile_n: usize) -> Frame {
    assert!(bank <= 1);

    let mut frame = Frame::new();
    let bank = (bank * 0x1000 )as usize;

    let tile = &chr_rom[(bank + tile_n * 16)..(bank + tile_n * 16 + 15)];

    for y in 0..=7 {
        let mut upper = tile[y];
        let mut lower = tile[y + 8];

        for x in (0..=7).rev() {
            let val = (1 & upper) << 1 | (1 & lower);

            upper >>= 1;
            lower >>= 1;
            let rgb = match val {
                0 => palette::SYSTEM_PALLETE[0x01],
               1 => palette::SYSTEM_PALLETE[0x23],
               2 => palette::SYSTEM_PALLETE[0x27],
               3 => palette::SYSTEM_PALLETE[0x30],
               _ => panic!("Invalid!"),
            };
            frame.set_pixel(x, y, rgb);
        }
    }

    frame
}