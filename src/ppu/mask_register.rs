pub struct MaskRegister {
    pub greyscale: bool,
    pub show_left_background: bool,
    pub show_left_sprites: bool,
    pub show_background: bool,
    pub show_sprites: bool,
    pub emphasize_red: bool,
    pub emphasize_green: bool,
    pub emphasize_blue: bool,
}

pub enum Colour {
    Red,
    Green,
    Blue,
}

impl MaskRegister {
    pub fn new() -> Self {
        MaskRegister {
            greyscale: false,
            show_left_background: false,
            show_left_sprites: false,
            show_background: false,
            show_sprites: false,
            emphasize_red: false,
            emphasize_green: false,
            emphasize_blue: false,
        }
    }

    pub fn emphasize(&self) -> Vec<Colour> {
        let mut res = Vec::new();

        if self.emphasize_red {
            res.push(Colour::Red);
        }
        if self.emphasize_green {
            res.push(Colour::Green);
        }
        if self.emphasize_blue {
            res.push(Colour::Blue);
        }

        res
    }

    pub fn update(&mut self, data: u8) {
        self.greyscale = data & 0b00000001 != 0;
        self.show_left_background = data & 0b00000010 != 0;
        self.show_left_sprites = data & 0b00000100 != 0;
        self.show_background = data & 0b00001000 != 0;
        self.show_sprites = data & 0b00010000 != 0;
        self.emphasize_red = data & 0b00100000 != 0;
        self.emphasize_green = data & 0b01000000 != 0;
        self.emphasize_blue = data & 0b10000000 != 0;
    }
}

 