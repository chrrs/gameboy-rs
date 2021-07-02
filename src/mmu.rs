use crate::cartridge::Cartridge;

pub struct Mmu {
    bios: Option<&'static [u8]>,
    cart: Cartridge,
    wram: Box<[u8; 0x7fff]>,
    hram: Box<[u8; 0x7e]>,
}

impl Mmu {
    pub fn new(bios: &'static [u8], cart: Cartridge) -> Mmu {
        Mmu {
            bios: Some(bios),
            cart,
            wram: Box::new([0; 0x7fff]),
            hram: Box::new([0; 0x7e]),
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
            0xc000..=0xdfff => self.wram[address as usize % 0x1fff],
            0xe000..=0xfdff => self.read(address - 0x2000),
            0xff80..=0xfffe => self.hram[address as usize % 0x7f],
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
            0xc000..=0xdfff => self.wram[address as usize % 0x1fff] = value,
            0xe000..=0xfdff => self.write(address - 0x2000, value),
            0xff80..=0xfffe => self.hram[address as usize % 0x7f] = value,
            _ => panic!("tried to write to unmapped memory at {:#x}", address),
        }
    }
}
