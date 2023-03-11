use super::ProcessingUnit;

pub struct Cpu {}

impl ProcessingUnit for Cpu {
    fn new() -> Self {
        return Self {};
    }

    fn step(&mut self) {}
}
