pub enum GpuMode {
    HBlank,
    VBlank,
    OamRead,
    VramRead,
}

pub struct Gpu {
    pub vram: Box<[u8; 0x2000]>,
    pub oam: Box<[u8; 0xa0]>,
    mode_cycles: usize,
    line: u8,
    mode: GpuMode,
    pub scroll_x: u8,
    pub scroll_y: u8,
}

impl Gpu {
    pub fn new() -> Gpu {
        Gpu {
            vram: Box::new([0; 0x2000]),
            oam: Box::new([0; 0xa0]),
            mode: GpuMode::HBlank,
            mode_cycles: 0,
            line: 0,
            scroll_x: 0,
            scroll_y: 0,
        }
    }

    pub fn scanline(&self) -> u8 {
        self.line
    }

    pub fn cycle(&mut self, cycles: usize) {
        self.mode_cycles += cycles;

        match self.mode {
            GpuMode::HBlank => {
                if self.mode_cycles >= 204 {
                    self.mode_cycles -= 204;
                    self.line += 1;

                    if self.line == 143 {
                        self.mode = GpuMode::VBlank;

                        // Output image
                    } else {
                        self.mode = GpuMode::OamRead;
                    }
                }
            }
            GpuMode::VBlank => {
                if self.mode_cycles >= 456 {
                    self.mode_cycles -= 456;
                    self.line += 1;

                    if self.line > 153 {
                        self.mode = GpuMode::OamRead;
                        self.line = 0;
                    }
                }
            }
            GpuMode::OamRead => {
                if self.mode_cycles >= 80 {
                    self.mode_cycles -= 80;
                    self.mode = GpuMode::VramRead;
                }
            }
            GpuMode::VramRead => {
                if self.mode_cycles >= 172 {
                    self.mode_cycles -= 172;
                    self.mode = GpuMode::HBlank;

                    // Write a scanline
                }
            }
        }
    }
}
