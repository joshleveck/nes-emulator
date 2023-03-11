mod cpu;
mod ppu;
mod apu;

trait ProcessingUnit {
    fn new() -> Self;
    fn step(&mut self);
}

pub struct Controller {
   master_clock: i64,
   cpu: cpu::Cpu,
   ppu: ppu::Ppu,
   apu: apu::Apu
}

impl Controller {
    pub fn new() -> Self {
        Self {
            master_clock: 0,
            cpu: cpu::Cpu::new(),
            ppu: ppu::Ppu::new(),
            apu: apu::Apu::new()
        }
    }

    pub fn master_loop(&mut self) {
        // Initiate CPU, PPU, APU

        loop {
            self.master_clock += 1;

            // Step
            self.cpu.step();
            self.apu.step();
            self.ppu.step();
        }
    }
}