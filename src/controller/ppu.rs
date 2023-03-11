use super::ProcessingUnit;

pub struct Ppu {}

impl ProcessingUnit for Ppu {
    fn new() -> Self {
        return Self {};
    }

    fn step(&mut self) {}
}
