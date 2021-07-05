use crate::{cartridge::Cartridge, gpu::Gpu};

use super::{Memory, MemoryError, MemoryOperation};

pub struct Mmu {
    bios: Option<&'static [u8]>,
    pub cart: Cartridge,
    pub gpu: Gpu,
    wram: Box<[u8; 0x2000]>,
    hram: Box<[u8; 0x7f]>,
}

impl Mmu {
    pub fn new(bios: &'static [u8], cart: Cartridge, gpu: Gpu) -> Mmu {
        Mmu {
            bios: Some(bios),
            cart,
            gpu,
            wram: Box::new([0; 0x2000]),
            hram: Box::new([0; 0x7f]),
        }
    }
}

impl Memory for Mmu {
    fn read(&self, address: u16) -> Result<u8, MemoryError> {
        match address {
            0..=0xff => {
                if let Some(bios) = self.bios {
                    Ok(bios[address as usize])
                } else {
                    Ok(self.cart.read(address))
                }
            }
            0x100..=0x7fff => Ok(self.cart.read(address)),
            0x8000..=0x9fff => Ok(self.gpu.vram[address as usize - 0x8000]),
            0xc000..=0xdfff => Ok(self.wram[address as usize - 0xc000]),
            0xe000..=0xfdff => self.read(address - 0x2000),
            0xfe00..=0xfe9f => Ok(self.gpu.oam[address as usize - 0xfe00]),
            0xfea0..=0xfeff => Ok(0xff),
            0xff00 => Ok(0xff), // Joypad P1
            0xff42 => Ok(self.gpu.scroll_y),
            0xff43 => Ok(self.gpu.scroll_x),
            0xff44 => Ok(self.gpu.scanline()),
            0xff80..=0xfffe => Ok(self.hram[address as usize - 0xff80]),
            _ => Err(MemoryError::Unmapped {
                address,
                op: MemoryOperation::Read,
            }),
        }
    }

    fn write(&mut self, address: u16, value: u8) -> Result<(), MemoryError> {
        match address {
            0..=0xff => {
                if self.bios.is_some() {
                    Err(MemoryError::Illegal {
                        address,
                        op: MemoryOperation::Write,
                    })
                } else {
                    self.cart.write(address, value);
                    Ok(())
                }
            }
            0x100..=0x7fff => {
                self.cart.write(address, value);
                Ok(())
            }
            0x8000..=0x9fff => {
                self.gpu.vram[address as usize - 0x8000] = value;
                self.gpu.update_tile(address - 0x8000);
                Ok(())
            }
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
            0xff00 => Ok(()),          // Joypad P1
            0xff01 => Ok(()),          // Serial transfer data
            0xff02 => Ok(()),          // Serial transfer control
            0xff06 => Ok(()),          // Timer Modulo
            0xff0f => Ok(()),          // Interrupt flag
            0xff10..=0xff26 => Ok(()), // Sound
            0xff40 => Ok(()),          // LCD Control
            0xff41 => Ok(()),          // LCD Stat
            0xff42 => {
                self.gpu.scroll_y = value;
                Ok(())
            }
            0xff43 => {
                self.gpu.scroll_x = value;
                Ok(())
            }
            0xff44 => Err(MemoryError::ReadOnly { address }),
            0xff47 => Ok(()), // BG Palette Data
            0xff48 => Ok(()), // Object Palette 0 Data
            0xff49 => Ok(()), // Object Palette 1 Data
            0xff4a => Ok(()), // Window Y
            0xff4b => Ok(()), // Window X
            0xff50 => {
                if value != 0 {
                    self.bios = None;
                }

                Ok(())
            }
            0xff70..=0xff7f => Ok(()), // WRAM Bank Select
            0xff80..=0xfffe => {
                self.hram[address as usize - 0xff80] = value;
                Ok(())
            }
            0xffff => Ok(()), // Enable interrupts
            _ => Err(MemoryError::Unmapped {
                address,
                op: MemoryOperation::Write,
            }),
        }
    }
}
