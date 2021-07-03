use crate::{cartridge::Cartridge, gpu::Gpu};

pub struct Mmu {
    bios: Option<&'static [u8]>,
    cart: Cartridge,
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

    pub fn read(&self, address: u16) -> u8 {
        match address {
            0..=0xff => {
                if let Some(bios) = self.bios {
                    bios[address as usize]
                } else {
                    self.cart.read(address)
                }
            }
            0x100..=0x7fff => self.cart.read(address),
            0x8000..=0x9fff => self.gpu.vram[address as usize - 0x8000],
            0xc000..=0xdfff => self.wram[address as usize - 0xc000],
            0xe000..=0xfdff => self.read(address - 0x2000),
            0xfe00..=0xfe9f => self.gpu.oam[address as usize - 0xfe00],
            0xff42 => self.gpu.scroll_y,
            0xff43 => self.gpu.scroll_x,
            0xff44 => self.gpu.scanline(),
            0xff80..=0xfffe => self.hram[address as usize - 0xff80],
            _ => panic!("tried to read from unmapped memory at {:#x}", address),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0..=0xff => {
                if self.bios.is_some() {
                    panic!(
                        "trying to write {:#x} into BIOS at address {:#x}",
                        value, address
                    )
                } else {
                    self.cart.write(address, value)
                }
            }
            0x100..=0x7fff => self.cart.write(address, value),
            0x8000..=0x9fff => self.gpu.vram[address as usize - 0x8000] = value,
            0xc000..=0xdfff => self.wram[address as usize - 0xc000] = value,
            0xe000..=0xfdff => self.write(address - 0x2000, value),
            0xfe00..=0xfe9f => self.gpu.oam[address as usize - 0xfe00] = value,
            0xff42 => self.gpu.scroll_y = value,
            0xff43 => self.gpu.scroll_x = value,
            0xff44 => panic!("tried to write to scanline field in memory"),
            0xff80..=0xfffe => self.hram[address as usize - 0xff80] = value,
            _ => println!("tried to write to unmapped memory at {:#x}", address),
        }
    }
}
