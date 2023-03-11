use super::ProcessingUnit;

pub struct Apu {}

impl ProcessingUnit for Apu {
    fn new() -> Self {
        return Self {};
    }

    fn step(&mut self) {}
}
