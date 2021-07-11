use std::collections::BTreeMap;

use anyhow::Context;

use crate::{
    bios::DMG_BIOS,
    cartridge::Cartridge,
    cpu::Cpu,
    gpu::Gpu,
    memory::mmu::{JoypadButton, Mmu},
};

#[cfg(feature = "dump-log")]
use crate::memory::Memory;
#[cfg(feature = "dump-log")]
use std::{fs::File, io::Write};

const PALETTE: [[u8; 3]; 4] = [[255, 255, 255], [192, 192, 192], [96, 96, 96], [0, 0, 0]];

pub struct Device {
    cpu: Cpu,
    mmu: Mmu,

    tile_framebuffer: Box<[u8; 3 * 16 * 24 * 8 * 8]>,
    display_framebuffer: Box<[u8; 3 * 160 * 144]>,

    #[cfg(feature = "dump-log")]
    log: File,
}

impl Device {
    pub fn new(cart: Cartridge) -> Device {
        Device {
            cpu: Cpu::new(),
            mmu: Mmu::new(DMG_BIOS, cart, Gpu::new()),
            tile_framebuffer: Box::new([0; 3 * 16 * 24 * 8 * 8]),
            display_framebuffer: Box::new([0; 3 * 160 * 144]),

            #[cfg(feature = "dump-log")]
            log: File::create("log.txt").expect("cannot create dump log file"),
        }
    }

    pub fn reset(&mut self) {
        self.cpu.reset();
        self.mmu.gpu.reset();
        self.mmu.use_bios = true;
    }

    pub fn step_frame(&mut self) {
        while !self.step() {}
    }

    pub fn step_frame_until_pc(&mut self, pc: u16) {
        while !self.step() && self.cpu.pc != pc {}
    }

    pub fn step(&mut self) -> bool {
        #[cfg(feature = "dump-log")]
        let Device { cpu, mmu, log, .. } = self;

        #[cfg(feature = "dump-log")]
        writeln!(log, "A: {:02X} F: {:02X} B: {:02X} C: {:02X} D: {:02X} E: {:02X} H: {:02X} L: {:02X} SP: {:04X} PC: {:02X}:{:04X} ({:02X} {:02X} {:02X} {:02X})",
            cpu.a, cpu.f, cpu.b, cpu.c, cpu.d, cpu.e, cpu.h, cpu.l, cpu.sp, 0, cpu.pc, mmu.read(cpu.pc).unwrap(), mmu.read(cpu.pc + 1).unwrap(), mmu.read(cpu.pc + 2).unwrap(), mmu.read(cpu.pc + 3).unwrap())
            .unwrap();

        #[cfg(not(feature = "dump-log"))]
        let Device { cpu, mmu, .. } = self;

        if mmu.step(cpu) {
            self.update_framebuffers();
            true
        } else {
            false
        }
    }

    pub fn skip(&mut self) {
        let Device { cpu, mmu, .. } = self;
        cpu.fetch_instruction(mmu)
            .context("failed to fetch next instruction")
            .unwrap();
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

    pub fn press(&mut self, buttons: &[JoypadButton]) {
        self.mmu.press(buttons);
    }

    pub fn release(&mut self, buttons: &[JoypadButton]) {
        self.mmu.release(buttons);
    }

    fn update_framebuffers(&mut self) {
        for tile_x in 0..16 {
            for tile_y in 0..24 {
                let tile = self.gpu().tiles[tile_x + tile_y * 16];

                for x in 0..8 {
                    for y in 0..8 {
                        let color =
                            PALETTE[self.gpu().bg_palette[tile.get(x, y) as usize] as usize];

                        let index = 3 * (8 * tile_x + x + 16 * 8 * 8 * tile_y + 16 * 8 * y);
                        for (i, c) in color.iter().enumerate() {
                            self.tile_framebuffer[index + i] = *c;
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
