use std::{
    fs::File,
    io::{self, BufReader, Read},
};

const LOGO: [u8; 0x30] = [
    0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D,
    0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99,
    0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E,
];

pub struct Cartridge {
    bytes: Vec<u8>,
}

impl Cartridge {
    pub fn new(file: File) -> Result<Cartridge, io::Error> {
        let mut reader = BufReader::new(file);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;

        Ok(Cartridge { bytes: buffer })
    }

    pub fn title(&self) -> Option<&str> {
        std::str::from_utf8(&self.bytes[0x134..=0x143]).ok()
    }

    pub fn verify(&self) -> bool {
        self.bytes[0x104..=0x133] == LOGO && self.verify_header_checksum()
    }

    pub fn read(&self, address: u16) -> u8 {
        self.bytes[address as usize]
    }

    pub fn write(&mut self, _address: u16, _value: u8) {
        // self.bytes[address as usize] = value
    }

    fn verify_header_checksum(&self) -> bool {
        let mut x = 0u8;

        for i in 0x134..=0x14c {
            x = x.wrapping_sub(self.bytes[i] + 1);
        }

        x == self.bytes[0x14d]
    }
}
