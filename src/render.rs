pub mod frame;
pub mod palette;

use crate::{cartridge::Mirroring, ppu::Ppu};
use frame::Frame;

struct Rect {
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
}

impl Rect {
    pub fn new(x1: usize, y1: usize, x2: usize, y2: usize) -> Self {
        Rect { x1, y1, x2, y2 }
    }
}

fn bg_palette(ppu: &Ppu, attribute_table: &[u8], tile_col: usize, tile_row: usize) -> [u8; 4] {
    let addr_table_idx = tile_row / 4 * 8 + tile_col / 4;
    let attr_byte = attribute_table[addr_table_idx];

    let palette_idx = match (tile_col % 4 / 2, tile_row % 4 / 2) {
        (0, 0) => attr_byte & 0b00000011,
        (1, 0) => (attr_byte & 0b00001100) >> 2,
        (0, 1) => (attr_byte & 0b00110000) >> 4,
        (1, 1) => (attr_byte & 0b11000000) >> 6,
        _ => panic!("Invalid!"),
    };

    let palette_start: usize = 1 + (palette_idx as usize) * 4;
    [
        ppu.get_palette_table(0),
        ppu.get_palette_table(palette_start),
        ppu.get_palette_table(palette_start + 1),
        ppu.get_palette_table(palette_start + 2),
    ]
}

fn sprite_palette(ppu: &Ppu, pallete_idx: u8) -> [u8; 4] {
    let start = 0x11 + (pallete_idx * 4) as usize;
    [
        0,
        ppu.get_palette_table(start),
        ppu.get_palette_table(start + 1),
        ppu.get_palette_table(start + 2),
    ]
}

fn render_name_table(
    ppu: &Ppu,
    frame: &mut Frame,
    name_table: &[u8],
    view_port: Rect,
    shift_x: isize,
    shift_y: isize,
) {
    let bank = ppu.ctrl_bknd();

    let attribute_table = &name_table[0x3c0..0x400];

    for i in 0..0x3c0 {
        let tile_column = i % 32;
        let tile_row = i / 32;
        let tile_idx = name_table[i] as u16;
        let tile = &ppu.get_chr_rom(bank + tile_idx * 16, bank + tile_idx * 16 + 15);
        let palette = bg_palette(ppu, attribute_table, tile_column, tile_row);

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];

            for x in (0..=7).rev() {
                let value = (1 & lower) << 1 | (1 & upper);
                upper = upper >> 1;
                lower = lower >> 1;
                let rgb = match value {
                    0 => palette::SYSTEM_PALLETE[ppu.get_palette_table(0) as usize],
                    1 => palette::SYSTEM_PALLETE[palette[1] as usize],
                    2 => palette::SYSTEM_PALLETE[palette[2] as usize],
                    3 => palette::SYSTEM_PALLETE[palette[3] as usize],
                    _ => panic!("Invalid"),
                };
                let pixel_x = tile_column * 8 + x;
                let pixel_y = tile_row * 8 + y;

                if pixel_x >= view_port.x1
                    && pixel_x < view_port.x2
                    && pixel_y >= view_port.y1
                    && pixel_y < view_port.y2
                {
                    frame.set_pixel(
                        (shift_x + pixel_x as isize) as usize,
                        (shift_y + pixel_y as isize) as usize,
                        rgb,
                    );
                }
            }
        }
    }
}

pub fn render(ppu: &Ppu, frame: &mut Frame) {
    let scroll_x = (ppu.get_scroll().x) as usize;
    let scroll_y = (ppu.get_scroll().y) as usize;

    let (main_nametable, second_nametable) = match (&ppu.get_mirroring(), ppu.get_ctrl_nametable())
    {
        (Mirroring::Vertical, 0x2000)
        | (Mirroring::Vertical, 0x2800)
        | (Mirroring::Horizontal, 0x2000)
        | (Mirroring::Horizontal, 0x2400) => {
            (ppu.get_vram_span(0, 0x400), ppu.get_vram_span(0x400, 0x800))
        }
        (Mirroring::Vertical, 0x2400)
        | (Mirroring::Vertical, 0x2C00)
        | (Mirroring::Horizontal, 0x2800)
        | (Mirroring::Horizontal, 0x2C00) => {
            (ppu.get_vram_span(0x400, 0x800), ppu.get_vram_span(0, 0x400))
        }
        (_, _) => {
            panic!("Not supported mirroring type {:?}", ppu.get_mirroring());
        }
    };

    render_name_table(
        ppu,
        frame,
        main_nametable,
        Rect::new(scroll_x, scroll_y, 256, 240),
        -(scroll_x as isize),
        -(scroll_y as isize),
    );
    if scroll_x > 0 {
        render_name_table(
            ppu,
            frame,
            second_nametable,
            Rect::new(0, 0, scroll_x, 240),
            (256 - scroll_x) as isize,
            0,
        );
    } else if scroll_y > 0 {
        render_name_table(
            ppu,
            frame,
            second_nametable,
            Rect::new(0, 0, 256, scroll_y),
            0,
            (240 - scroll_y) as isize,
        );
    }

    let oam_data = ppu.get_oam_data().to_vec();

    for i in (0..oam_data.len()).step_by(4).rev() {
        let tile_idx = oam_data[i + 1] as u16;
        let tile_x = oam_data[i + 3] as usize;
        let tile_y = oam_data[i] as usize;

        let flip_vert = if oam_data[i + 2] & 0b10000000 != 0 {
            true
        } else {
            false
        };

        let flip_horz = if oam_data[i + 2] & 0b01000000 != 0 {
            true
        } else {
            false
        };

        let palette_idx = oam_data[i + 2] & 0b11;
        let sprite_palette = sprite_palette(ppu, palette_idx);

        let bank: u16 = ppu.ctrl_sptr();
        let tile = &ppu.get_chr_rom(bank + tile_idx * 16, bank + tile_idx * 16 + 15);

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];
            'x: for x in (0..=7).rev() {
                let val = (1 & lower) << 1 | (1 & upper); // THIS IS WEIRD
                upper >>= 1;
                lower >>= 1;
                let rgb = match val {
                    0 => continue 'x,
                    1 => palette::SYSTEM_PALLETE[sprite_palette[1] as usize],
                    2 => palette::SYSTEM_PALLETE[sprite_palette[2] as usize],
                    3 => palette::SYSTEM_PALLETE[sprite_palette[3] as usize],
                    _ => panic!("Invalid!"),
                };

                match (flip_horz, flip_vert) {
                    (false, false) => frame.set_pixel(tile_x + x, tile_y + y, rgb),
                    (true, false) => frame.set_pixel(tile_x + 7 - x, tile_y + y, rgb),
                    (false, true) => frame.set_pixel(tile_x + x, tile_y + 7 - y, rgb),
                    (true, true) => frame.set_pixel(tile_x + 7 - x, tile_y + 7 - y, rgb),
                }
            }
        }
    }
}
