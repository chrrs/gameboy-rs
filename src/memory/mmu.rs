use crate::{cpu::Interrupts, timer::Timer};
use anyhow::Context;

use crate::{
    cartridge::Cartridge,
    cpu::Cpu,
    gpu::{Gpu, LcdControl},
};

use super::{Memory, MemoryError, MemoryOperation};

pub struct Mmu {
    bios: &'static [u8],
    pub use_bios: bool,
    pub cart: Cartridge,
    pub gpu: Gpu,
    pub timer: Timer,
    wram: Box<[u8; 0x2000]>,
    hram: Box<[u8; 0x7f]>,
    interrupts: Interrupts,
    interrupts_enabled: Interrupts,
}

impl Mmu {
    pub fn new(bios: &'static [u8], cart: Cartridge, gpu: Gpu) -> Mmu {
        Mmu {
            bios,
            use_bios: true,
            cart,
            gpu,
            timer: Timer::new(),
            wram: Box::new([0; 0x2000]),
            hram: Box::new([0; 0x7f]),
            interrupts: Interrupts::empty(),
            interrupts_enabled: Interrupts::empty(),
        }
    }

    pub fn step(&mut self, cpu: &mut Cpu) -> bool {
        let cycles = if cpu.halted {
            4
        } else {
            cpu.exec_next_instruction(self)
                .context("failed to execute next instruction")
                .unwrap()
        };

        let (frame, new_interrupts) = self.gpu.cycle(4 * cycles);
        self.interrupts.insert(new_interrupts);

        let new_interrupts = self.timer.cycle(cycles);
        self.interrupts.insert(new_interrupts);

        let mut to_process_interrupts = self.interrupts;
        to_process_interrupts.remove(!self.interrupts_enabled);

        if !to_process_interrupts.is_empty() {
            cpu.halted = false;
        }

        let (cycles, handled_interrupts) = cpu.process_interrupts(self, to_process_interrupts);
        self.interrupts.remove(handled_interrupts);

        if cycles != 0 {
            let (frame2, new_interrupts) = self.gpu.cycle(4 * cycles);
            self.interrupts.insert(new_interrupts);

            let new_interrupts = self.timer.cycle(cycles);
            self.interrupts.insert(new_interrupts);

            return frame || frame2;
        }

        frame
    }
}

impl Memory for Mmu {
    fn read(&self, address: u16) -> Result<u8, MemoryError> {
        match address {
            0..=0xff if self.use_bios => Ok(self.bios[address as usize]),
            0..=0x7fff => self.cart.read(address),
            0x8000..=0x9fff => Ok(self.gpu.vram[address as usize - 0x8000]),
            0xa000..=0xbfff => self.cart.read(address),
            0xc000..=0xdfff => Ok(self.wram[address as usize - 0xc000]),
            0xe000..=0xfdff => self.read(address - 0x2000),
            0xfe00..=0xfe9f => Ok(self.gpu.oam[address as usize - 0xfe00]),
            0xfea0..=0xfeff => Ok(0xff),
            0xff00 => Ok(0xff), // Joypad P1
            0xff04 => Ok(self.timer.divider),
            0xff05 => Ok(self.timer.counter),
            0xff06 => Ok(self.timer.modulo),
            0xff07 => Ok(self.timer.timer_control()),
            0xff0f => Ok(self.interrupts.bits()),
            0xff10..=0xff26 => Ok(0), // Sound
            0xff30..=0xff3f => Ok(0), // Wave Pattern RAM
            0xff40 => Ok(self.gpu.lcd_control.bits()),
            0xff41 => Ok(self.gpu.stat()),
            0xff42 => Ok(self.gpu.scroll_y),
            0xff43 => Ok(self.gpu.scroll_x),
            0xff44 => Ok(self.gpu.scanline()),
            0xff45 => Ok(self.gpu.lyc),
            0xff47 => Ok(pack_palette(self.gpu.bg_palette)),
            0xff4d => Ok(0xff),
            0xff80..=0xfffe => Ok(self.hram[address as usize - 0xff80]),
            0xffff => Ok(self.interrupts_enabled.bits()),
            _ => Err(MemoryError::Unmapped {
                address,
                op: MemoryOperation::Read,
            }),
        }
    }

    fn write(&mut self, address: u16, value: u8) -> Result<(), MemoryError> {
        match address {
            0..=0xff if self.use_bios => Err(MemoryError::Illegal {
                address,
                op: MemoryOperation::Write,
            }),
            0..=0x7fff => self.cart.write(address, value),
            0x8000..=0x9fff => {
                self.gpu.vram[address as usize - 0x8000] = value;
                self.gpu.update_tile(address - 0x8000);
                Ok(())
            }
            0xa000..=0xbfff => self.cart.write(address, value),
            0xc000..=0xdfff => {
                self.wram[address as usize - 0xc000] = value;
                Ok(())
            }
            0xe000..=0xfdff => self.write(address - 0x2000, value),
            0xfe00..=0xfe9f => {
                self.gpu.oam[address as usize - 0xfe00] = value;
                Ok(())
            }
            0xfea0..=0xfeff => Ok(()),
            0xff00 => Ok(()), // Joypad P1
            0xff01 => Ok(()), // Serial transfer data
            0xff02 => Ok(()), // Serial transfer control
            0xff04 => {
                self.timer.divider = 0;
                self.timer.counter = 0;
                Ok(())
            }
            0xff05 => {
                self.timer.counter = value;
                Ok(())
            }
            0xff06 => {
                self.timer.modulo = value;
                Ok(())
            }
            0xff07 => {
                self.timer.set_timer_control(value);
                Ok(())
            }
            0xff0f => {
                self.interrupts = Interrupts::from_bits_truncate(value);
                Ok(())
            } // Interrupt flag
            0xff10..=0xff26 => Ok(()), // Sound
            0xff30..=0xff3f => Ok(()), // Wave Pattern RAM
            0xff40 => {
                self.gpu.lcd_control = LcdControl::from_bits_truncate(value);
                Ok(())
            }
            0xff41 => {
                self.gpu.set_stat(value);
                Ok(())
            }
            0xff42 => {
                self.gpu.scroll_y = value;
                Ok(())
            }
            0xff43 => {
                self.gpu.scroll_x = value;
                Ok(())
            }
            0xff44 => Err(MemoryError::ReadOnly { address }),
            0xff45 => {
                self.gpu.lyc = value;
                Ok(())
            }
            0xff46 => Ok(()), // DMA Transfer
            0xff47 => {
                self.gpu.bg_palette = unpack_palette(value);
                Ok(())
            }
            0xff48 => Ok(()), // Object Palette 0 Data
            0xff49 => Ok(()), // Object Palette 1 Data
            0xff4a => Ok(()), // Window Y
            0xff4b => Ok(()), // Window X
            0xff4d => Ok(()), // GBC Speed switch
            0xff50 => {
                if value != 0 {
                    self.use_bios = false;
                }

                Ok(())
            }
            0xff70..=0xff7f => Ok(()), // WRAM Bank Select
            0xff80..=0xfffe => {
                self.hram[address as usize - 0xff80] = value;
                Ok(())
            }
            0xffff => {
                self.interrupts_enabled = Interrupts::from_bits_truncate(value);
                Ok(())
            }
            _ => Err(MemoryError::Unmapped {
                address,
                op: MemoryOperation::Write,
            }),
        }
    }
}

pub fn pack_palette(palette: [u8; 4]) -> u8 {
    let mut value = 0;

    for (i, el) in palette.iter().enumerate() {
        value |= el << (i * 2);
    }

    value
}

pub fn unpack_palette(palette: u8) -> [u8; 4] {
    let mut value = [0; 4];

    for (i, el) in value.iter_mut().enumerate() {
        *el = (palette & (0b11 << (i * 2))) >> (i * 2);
    }

    value
}
