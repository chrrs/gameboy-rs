use bitflags::bitflags;

bitflags! {
    pub struct LcdControl: u8 {
        const BG_WINDOW_ENABLE = 1 << 0; // TODO: Partially
        const OBJ_ENABLE = 1 << 1; // TODO
        const OBJ_SIZE = 1 << 2; // TODO
        const BG_TILEMAP_AREA = 1 << 3;
        const BG_WINDOW_TILEDATA_AREA = 1 << 4; // TODO: Partially
        const WINDOW_ENABLE = 1 << 5; // TODO
        const WINDOW_TILEMAP_AREA = 1 << 6; // TODO
        const LCD_ENABLE = 1 << 7;
    }
}

pub enum GpuMode {
    HBlank,
    VBlank,
    OamRead,
    VramRead,
}

#[derive(Clone, Copy)]
pub struct Tile {
    pixels: [u8; 64],
}

impl Tile {
    pub fn new() -> Tile {
        Tile { pixels: [0; 64] }
    }

    pub fn set(&mut self, x: usize, y: usize, value: u8) {
        self.pixels[x + y * 8] = value;
    }

    pub fn get(&self, x: usize, y: usize) -> u8 {
        self.pixels[x + y * 8]
    }
}

pub struct Gpu {
    pub vram: Box<[u8; 0x2000]>,
    pub oam: Box<[u8; 0xa0]>,
    mode_cycles: usize,
    line: u8,
    mode: GpuMode,
    pub scroll_x: u8,
    pub scroll_y: u8,
    pub tiles: Box<[Tile; 384]>,
    pub framebuffer: Box<[u8; 160 * 144]>,
    pub lcd_control: LcdControl,
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
            tiles: Box::new([Tile::new(); 384]),
            framebuffer: Box::new([0; 160 * 144]),
            lcd_control: LcdControl::empty(),
        }
    }

    pub fn reset(&mut self) {
        self.scroll_x = 0;
        self.scroll_y = 0;
        self.line = 0;
        self.mode = GpuMode::HBlank;
        self.mode_cycles = 0;
    }

    pub fn scanline(&self) -> u8 {
        self.line
    }

    pub fn cycle(&mut self, cycles: usize) -> bool {
        self.mode_cycles += cycles;

        match self.mode {
            GpuMode::HBlank => {
                if self.mode_cycles >= 204 {
                    self.mode_cycles -= 204;
                    self.line += 1;

                    if self.line == 143 {
                        self.mode = GpuMode::VBlank;
                        return true;
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

                    self.render_scanline();
                }
            }
        }

        false
    }

    pub fn update_tile(&mut self, vram_address: u16) {
        let vram_address = vram_address & !1;

        let tile = vram_address / 16;

        if tile >= 384 {
            return;
        }

        let y = vram_address % 16 / 2;

        for x in 0..8 {
            let bit = 1 << (7 - x);

            let mut value = if self.vram[vram_address as usize] & bit != 0 {
                1
            } else {
                0
            };

            if self.vram[vram_address as usize + 1] & bit != 0 {
                value += 2;
            }

            self.tiles[tile as usize].set(x, y as usize, value)
        }
    }

    fn render_scanline(&mut self) {
        if !self.lcd_control.contains(LcdControl::LCD_ENABLE) {
            self.framebuffer.fill(0);
            return;
        }

        if !self.lcd_control.contains(LcdControl::BG_WINDOW_ENABLE) {
            self.framebuffer.fill(0);
        } else {
            self.render_background();
        }
    }

    fn render_background(&mut self) {
        let mut address = if self.lcd_control.contains(LcdControl::BG_TILEMAP_AREA) {
            0x1c00
        } else {
            0x1800
        };

        address += (self.line.wrapping_add(self.scroll_y) as usize) / 8 * 32;
        address += (self.scroll_x / 8) as usize;

        let tile_y = self.line.wrapping_add(self.scroll_y) % 8;

        let mut tile = self.vram[address] as usize;
        address += 1;

        if !self
            .lcd_control
            .contains(LcdControl::BG_WINDOW_TILEDATA_AREA)
            && tile < 128
        {
            tile += 256;
        }

        let mut tile_x = self.scroll_x % 8;
        for x in 0..160 {
            let index = x + 160 * self.line as usize;
            self.framebuffer[index] = self.tiles[tile].get(tile_x as usize, tile_y as usize);

            tile_x += 1;
            if tile_x == 8 {
                tile_x = 0;
                tile = self.vram[address] as usize;
                address += 1;

                if !self
                    .lcd_control
                    .contains(LcdControl::BG_WINDOW_TILEDATA_AREA)
                    && tile < 128
                {
                    tile += 256;
                }
            }
        }
    }
}
