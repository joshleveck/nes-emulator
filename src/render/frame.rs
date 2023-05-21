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
