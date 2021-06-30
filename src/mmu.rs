use crate::cartridge::Cartridge;

pub struct Mmu {
    bios: Option<&'static [u8]>,
    cart: Cartridge,
}

impl Mmu {
    pub fn new(bios: &'static [u8], cart: Cartridge) -> Mmu {
        Mmu {
            bios: Some(bios),
            cart,
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
            _ => panic!("trying to read from unmapped address {:#x}", address),
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
            _ => panic!(
                "trying to write {:#x} to unmapped address {:#x}",
                value, address
            ),
        }
    }
}
