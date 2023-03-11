pub struct Memory {
    mem: [i16; 0xFFFF]
}

impl Memory {
    pub fn new() -> Self {
        return Self {}
    }

    pub fn get_value(&self, addr: i16) -> i16 {
        return self.mem[addr];
    }
}

