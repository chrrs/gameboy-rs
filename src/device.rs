use std::collections::BTreeMap;

use crate::{bios::DMG_BIOS, cartridge::Cartridge, cpu::Cpu, gpu::Gpu, mmu::Mmu};

const PALETTE: [[u8; 3]; 4] = [[255, 255, 255], [192, 192, 192], [96, 96, 96], [0, 0, 0]];

pub struct Device {
    cpu: Cpu,
    mmu: Mmu,

    tile_framebuffer: Box<[u8; 3 * 16 * 24 * 8 * 8]>,
    display_framebuffer: Box<[u8; 3 * 160 * 144]>,
}

impl Device {
    pub fn new(cart: Cartridge) -> Device {
        Device {
            cpu: Cpu::new(),
            mmu: Mmu::new(DMG_BIOS, cart, Gpu::new()),
            tile_framebuffer: Box::new([0; 3 * 16 * 24 * 8 * 8]),
            display_framebuffer: Box::new([0; 3 * 160 * 144]),
        }
    }

    pub fn reset(&mut self) {
        self.cpu.a = 0;
        self.cpu.b = 0;
        self.cpu.c = 0;
        self.cpu.d = 0;
        self.cpu.e = 0;
        self.cpu.h = 0;
        self.cpu.l = 0;
        self.cpu.f = 0;
        self.cpu.pc = 0;
        self.mmu.gpu.scroll_x = 0;
        self.mmu.gpu.scroll_y = 0;
    }

    pub fn step_frame(&mut self) {
        while !self.step() {}
    }

    pub fn step_frame_until_pc(&mut self, pc: u16) {
        while !self.step() && self.cpu.pc != pc {}
    }

    pub fn step(&mut self) -> bool {
        let Device { cpu, mmu, .. } = self;
        let cycles = cpu.exec_next_instruction(mmu);

        if mmu.gpu.cycle(cycles) {
            self.update_framebuffers();
            true
        } else {
            false
        }
    }

    pub fn skip(&mut self) {
        let Device { cpu, mmu, .. } = self;
        cpu.fetch_instruction(mmu);
    }

    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    pub fn cpu_mut(&mut self) -> &mut Cpu {
        &mut self.cpu
    }

    pub fn gpu(&self) -> &Gpu {
        &self.mmu.gpu
    }

    pub fn cart(&self) -> &Cartridge {
        &self.mmu.cart
    }

    pub fn disassemble(&mut self, max: u16) -> BTreeMap<u16, String> {
        let Device { cpu, mmu, .. } = self;
        cpu.disassemble(mmu, max)
    }

    pub fn tile_framebuffer(&self) -> &[u8] {
        self.tile_framebuffer.as_ref()
    }

    pub fn display_framebuffer(&self) -> &[u8] {
        self.display_framebuffer.as_ref()
    }

    fn update_framebuffers(&mut self) {
        for tile_x in 0..16 {
            for tile_y in 0..24 {
                let tile = self.gpu().tiles[tile_x + tile_y * 16];

                for x in 0..8 {
                    for y in 0..8 {
                        let color = PALETTE[tile.get(x, y) as usize];

                        let index = 3 * (8 * tile_x + x + 16 * 8 * 8 * tile_y + 16 * 8 * y);
                        for i in 0..3 {
                            self.tile_framebuffer[index + i] = color[i];
                        }
                    }
                }
            }
        }

        let Device {
            mmu,
            display_framebuffer,
            ..
        } = self;

        let framebuffer = mmu.gpu.framebuffer.as_ref();
        for i in 0..framebuffer.len() {
            for c in 0..3 {
                display_framebuffer[i * 3 + c] = PALETTE[framebuffer[i] as usize][c];
            }
        }
    }
}
