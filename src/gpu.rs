use crate::cpu::Interrupts;
use bitflags::bitflags;

bitflags! {
    pub struct LcdControl: u8 {
        const BG_WINDOW_ENABLE = 1 << 0;
        const OBJ_ENABLE = 1 << 1;
        const OBJ_SIZE = 1 << 2;
        const BG_TILEMAP_AREA = 1 << 3;
        const BG_WINDOW_TILEDATA_AREA = 1 << 4;
        const WINDOW_ENABLE = 1 << 5;
        const WINDOW_TILEMAP_AREA = 1 << 6;
        const LCD_ENABLE = 1 << 7;
    }
}

bitflags! {
    pub struct StatInterruptSource: u8 {
        const HBLANK = 1 << 3;
        const VBLANK = 1 << 4;
        const OAM = 1 << 5;
        const LYC_LY = 1 << 6;
    }
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum GpuMode {
    HBlank = 0,
    VBlank = 1,
    OamRead = 2,
    VramRead = 3,
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
    pub lyc: u8,
    mode: GpuMode,
    pub scroll_x: u8,
    pub scroll_y: u8,
    pub tiles: Box<[Tile; 384]>,
    pub framebuffer: Box<[u8; 160 * 144]>,
    pub lcd_control: LcdControl,
    stat_interrupt_source: StatInterruptSource,
    pub bg_palette: [u8; 4],
    pub window_coords: (u8, u8),
    window_drawing: bool,
    window_line: usize,
}

impl Gpu {
    pub fn new() -> Gpu {
        Gpu {
            vram: Box::new([0; 0x2000]),
            oam: Box::new([0; 0xa0]),
            mode: GpuMode::HBlank,
            mode_cycles: 0,
            line: 0,
            lyc: 0,
            scroll_x: 0,
            scroll_y: 0,
            tiles: Box::new([Tile::new(); 384]),
            framebuffer: Box::new([0; 160 * 144]),
            lcd_control: LcdControl::empty(),
            stat_interrupt_source: StatInterruptSource::empty(),
            bg_palette: [0; 4],
            window_coords: (0, 0),
            window_drawing: false,
            window_line: 0,
        }
    }

    pub fn reset(&mut self) {
        self.scroll_x = 0;
        self.scroll_y = 0;
        self.line = 0;
        self.mode = GpuMode::HBlank;
        self.mode_cycles = 0;
    }

    pub fn stat(&self) -> u8 {
        let mut value = self.stat_interrupt_source.bits();
        value |= self.mode as u8;

        if self.line == self.lyc {
            value |= 1 << 2;
        }

        value
    }

    pub fn set_stat(&mut self, value: u8) {
        self.stat_interrupt_source = StatInterruptSource::from_bits_truncate(value);
    }

    pub fn scanline(&self) -> u8 {
        self.line
    }

    pub fn cycle(&mut self, cycles: usize) -> (bool, Interrupts) {
        self.mode_cycles += cycles;

        let mut new_interrupts = Interrupts::empty();

        match self.mode {
            GpuMode::HBlank => {
                if self.mode_cycles >= 204 {
                    self.mode_cycles -= 204;
                    self.line += 1;

                    if self
                        .stat_interrupt_source
                        .contains(StatInterruptSource::LYC_LY)
                        && self.lyc == self.line
                    {
                        new_interrupts.insert(Interrupts::LCD_STAT);
                    }

                    if self.line > 143 {
                        self.mode = GpuMode::VBlank;

                        if self
                            .stat_interrupt_source
                            .contains(StatInterruptSource::VBLANK)
                        {
                            new_interrupts.insert(Interrupts::LCD_STAT);
                        }

                        new_interrupts.insert(Interrupts::VBLANK);

                        self.window_drawing = false;

                        return (true, new_interrupts);
                    } else {
                        self.mode = GpuMode::OamRead;

                        if self
                            .stat_interrupt_source
                            .contains(StatInterruptSource::OAM)
                        {
                            new_interrupts.insert(Interrupts::LCD_STAT);
                        }
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

                        if (self
                            .stat_interrupt_source
                            .contains(StatInterruptSource::LYC_LY)
                            && self.lyc == self.line)
                            || self
                                .stat_interrupt_source
                                .contains(StatInterruptSource::OAM)
                        {
                            new_interrupts.insert(Interrupts::LCD_STAT);
                        }
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

                    if self.window_coords.1 == self.line {
                        self.window_drawing = true;
                        self.window_line = 0;
                    }

                    self.render_scanline();

                    if self
                        .stat_interrupt_source
                        .contains(StatInterruptSource::HBLANK)
                    {
                        new_interrupts.insert(Interrupts::LCD_STAT);
                    }
                }
            }
        }

        (false, new_interrupts)
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
            self.render_background_scanline();
        }

        if self.lcd_control.contains(LcdControl::WINDOW_ENABLE) {
            self.render_window_scanline();
        }

        if self.lcd_control.contains(LcdControl::OBJ_ENABLE) {
            self.render_sprite_scanline();
        }
    }

    fn render_background_scanline(&mut self) {
        let mut address = if self.lcd_control.contains(LcdControl::BG_TILEMAP_AREA) {
            0x1c00
        } else {
            0x1800
        };

        address += (self.line.wrapping_add(self.scroll_y) as usize) / 8 * 32;
        let mut line_offset = (self.scroll_x / 8) as usize;

        let tile_y = self.line.wrapping_add(self.scroll_y) % 8;

        let mut tile = self.vram[address + line_offset] as usize;
        line_offset = (line_offset + 1) % 32;

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
            self.framebuffer[index] =
                self.bg_palette[self.tiles[tile].get(tile_x as usize, tile_y as usize) as usize];

            tile_x += 1;
            if tile_x == 8 {
                tile_x = 0;
                tile = self.vram[address + line_offset] as usize;
                line_offset = (line_offset + 1) % 32;

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

    fn render_window_scanline(&mut self) {
        if self.line < self.window_coords.1 {
            return;
        }

        if !self.window_drawing {
            return;
        }

        if !(0..=166).contains(&self.window_coords.0) || !(0..=143).contains(&self.window_coords.1)
        {
            return;
        }

        let mut address = if self.lcd_control.contains(LcdControl::WINDOW_TILEMAP_AREA) {
            0x1c00
        } else {
            0x1800
        };

        address += self.window_line / 8 * 32;

        let tile_y = self.window_line % 8;

        let mut tile = self.vram[address] as usize;
        address += 1;

        if !self
            .lcd_control
            .contains(LcdControl::BG_WINDOW_TILEDATA_AREA)
            && tile < 128
        {
            tile += 256;
        }

        let mut tile_x = 0;
        let real_x = self.window_coords.0.saturating_sub(7) as usize;
        for x in 0..160 - real_x {
            let index = x + real_x + 160 * self.line as usize;
            self.framebuffer[index] =
                self.bg_palette[self.tiles[tile].get(tile_x as usize, tile_y as usize) as usize];

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

        self.window_line += 1;
    }

    fn render_sprite_scanline(&mut self) {
        let large_sprites = self.lcd_control.contains(LcdControl::OBJ_SIZE);
        let sprite_height = if large_sprites { 16 } else { 8 };

        let mut indices = (0..40)
            .filter(|i| {
                self.line + 16 >= self.oam[i * 4]
                    && self.line + 16 < self.oam[i * 4] + sprite_height
            })
            .take(10)
            .collect::<Vec<usize>>();

        indices.sort_by_key(|i| self.oam[i * 4 + 1]);

        for i in indices.iter() {
            let tile_index = self.oam[i * 4 + 2] as usize;
            let sprite_y = self.oam[i * 4] as usize - 16;
            let sprite_x = self.oam[i * 4 + 1] as isize - 8;
            let attributes = self.oam[i * 4 + 3];

            let mut y = self.line as usize - sprite_y;

            let tile = self.tiles[if large_sprites {
                if y >= 8 {
                    y -= 8;
                    (tile_index & 0xfe) + 1
                } else {
                    tile_index & 0xfe
                }
            } else {
                tile_index
            }];

            let bg_priority = attributes & (1 << 7) != 0;

            for x in 0..8 {
                let pixel = tile.get(x, y);

                if pixel == 0 {
                    continue;
                }

                if (sprite_x + x as isize) < 0 {
                    continue;
                }

                let index = self.line as usize * 160 + (sprite_x + x as isize) as usize;
                if !bg_priority || self.framebuffer[index] == 0 {
                    self.framebuffer[index] = pixel;
                }
            }
        }
    }
}
