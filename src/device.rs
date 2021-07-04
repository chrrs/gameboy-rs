use std::collections::BTreeMap;

use crate::{bios::DMG_BIOS, cartridge::Cartridge, cpu::Cpu, gpu::Gpu, mmu::Mmu};

pub struct Device {
    cpu: Cpu,
    mmu: Mmu,
}

impl Device {
    pub fn new(cart: Cartridge) -> Device {
        Device {
            cpu: Cpu::new(),
            mmu: Mmu::new(DMG_BIOS, cart, Gpu::new()),
        }
    }

    pub fn step(&mut self) {
        let Device { cpu, mmu } = self;
        let cycles = cpu.exec_next_instruction(mmu);
        mmu.gpu.cycle(cycles);
    }

    pub fn skip(&mut self) {
        let Device { cpu, mmu } = self;
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
        let Device { cpu, mmu } = self;
        cpu.disassemble(mmu, max)
    }

    pub fn reset(&mut self) {
        self.cpu.pc = 0;
    }
}
