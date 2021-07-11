use std::{
    ffi::CStr,
    fs::{create_dir_all, File},
    io::{self, BufReader, Read, Write},
    path::Path,
};

use crate::memory::{Memory, MemoryError};
use anyhow::anyhow;

const LOGO: [u8; 0x30] = [
    0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D,
    0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99,
    0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E,
];

struct MBC1State {
    enable_ram: bool,
    ram_mode: bool,
    bank1: u8,
    bank2: u8,
}

impl MBC1State {
    pub fn new() -> MBC1State {
        MBC1State {
            enable_ram: false,
            ram_mode: false,
            bank1: 0b00001,
            bank2: 0b00,
        }
    }

    pub fn rom_offset(&self) -> (usize, usize) {
        let lower = if self.ram_mode { self.bank2 << 5 } else { 0 } as usize;
        let upper = ((self.bank2 << 5) | self.bank1) as usize;
        (0x4000 * lower, 0x4000 * upper)
    }

    pub fn ram_offset(&self) -> usize {
        let bank = if self.ram_mode {
            self.bank2 as usize
        } else {
            0
        };
        0x2000 * bank
    }
}

struct MBC3State {
    bank: u8,
    map_select: u8,
}

impl MBC3State {
    pub fn new() -> MBC3State {
        MBC3State {
            bank: 1,
            map_select: 0,
        }
    }
}

enum MBC {
    None,
    MBC1(MBC1State),
    MBC3(MBC3State),
}

pub struct Cartridge {
    bytes: Vec<u8>,
    ram: Vec<u8>,
    mbc: MBC,
}

impl Cartridge {
    pub fn new(file: File) -> Result<Cartridge, io::Error> {
        let mut reader = BufReader::new(file);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;

        let mbc = match buffer[0x147] {
            0x00 => MBC::None,
            0x01..=0x03 => MBC::MBC1(MBC1State::new()),
            0x13 => MBC::MBC3(MBC3State::new()),
            _ => panic!("unsupported MBC type {:#04x}", buffer[0x147]),
        };

        let ram_size = match buffer[0x149] {
            0x02 => 0x2000,
            0x03 => 4 * 0x2000,
            0x04 => 16 * 0x2000,
            0x05 => 8 * 0x2000,
            _ => 0,
        };

        Ok(Cartridge {
            bytes: buffer,
            mbc,
            ram: vec![0; ram_size],
        })
    }

    pub fn title(&self) -> Option<&str> {
        unsafe { CStr::from_ptr(&self.bytes[0x134] as *const u8 as *const _) }
            .to_str()
            .ok()
    }

    pub fn verify(&self) -> bool {
        self.bytes[0x104..=0x133] == LOGO && self.verify_header_checksum()
    }

    pub fn try_load(&mut self) {
        let file_name = format!(
            "saves/{}.sav",
            self.title().expect("game has invalid title")
        );

        let path = Path::new(&file_name);

        if path.exists() {
            self.load(File::open(path).expect("failed to open save file"));
        }
    }

    fn load(&mut self, file: File) {
        let mut reader = BufReader::new(file);
        reader
            .read_to_end(&mut self.ram)
            .expect("failed to read save file");
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let file_name = format!(
            "saves/{}.sav",
            self.title()
                .ok_or_else(|| anyhow!("game has invalid title"))?
        );

        create_dir_all("saves")?;

        let mut file = File::create(file_name)?;
        file.write_all(&self.ram)?;

        Ok(())
    }

    fn verify_header_checksum(&self) -> bool {
        let mut x = 0u8;

        for i in 0x134..=0x14c {
            x = x.wrapping_sub(self.bytes[i] + 1);
        }

        x == self.bytes[0x14d]
    }

    fn read_ram(&self, offset: usize, address: u16) -> u8 {
        if self.ram.is_empty() {
            0xff
        } else {
            let offset = (offset + (address as usize & 0x1ffff)) % self.ram.len();
            self.ram[offset]
        }
    }

    fn write_ram(&mut self, offset: usize, address: u16, value: u8) {
        if self.ram.is_empty() {
            return;
        }

        let offset = (offset + (address as usize & 0x1ffff)) % self.ram.len();
        self.ram[offset] = value
    }
}

impl Memory for Cartridge {
    fn read(&self, address: u16) -> Result<u8, MemoryError> {
        match self.mbc {
            MBC::None => Ok(self.bytes[address as usize]),
            MBC::MBC1(ref state) => match address {
                0x0000..=0x3fff => {
                    let (lower, _) = state.rom_offset();
                    Ok(self.bytes[(lower | (address as usize & 0x3fff)) % self.bytes.len()])
                }
                0x4000..=0x7fff => {
                    let (_, upper) = state.rom_offset();
                    Ok(self.bytes[(upper | (address as usize & 0x3fff)) % self.bytes.len()])
                }
                0xa000..=0xbfff if state.enable_ram => {
                    Ok(self.read_ram(state.ram_offset(), address))
                }
                _ => Ok(0xff),
            },
            MBC::MBC3(ref state) => match address {
                0x0000..=0x3fff => Ok(self.bytes[(address as usize & 0x3fff) % self.bytes.len()]),
                0x4000..=0x7fff => Ok(self.bytes[((0x4000 * state.bank as usize)
                    | (address as usize & 0x3fff))
                    % self.bytes.len()]),
                0xa000..=0xbfff if state.map_select <= 0x03 => {
                    Ok(self.read_ram(0x2000 * (state.map_select & 0b11) as usize, address))
                }
                _ => Ok(0xff),
            },
        }
    }

    fn write(&mut self, address: u16, value: u8) -> Result<(), MemoryError> {
        match self.mbc {
            MBC::None => {}
            MBC::MBC1(ref mut state) => match address {
                0x0000..=0x1fff => state.enable_ram = (value & 0xf) == 0xa,
                0x2000..=0x3fff => state.bank1 = if value & 0x1f == 0 { 1 } else { value & 0x1f },
                0x4000..=0x5fff => state.bank2 = value & 0b11,
                0x6000..=0x7fff => state.ram_mode = value & 0b1 == 1,
                0xa000..=0xbfff if state.enable_ram => {
                    let offset = state.ram_offset();
                    self.write_ram(offset, address, value)
                }
                _ => {}
            },
            MBC::MBC3(ref mut state) => match address {
                0x0000..=0x1fff => {}
                0x2000..=0x3fff => state.bank = if value == 0 { 1 } else { value },
                0x4000..=0x5fff => state.map_select = value & 0b1111,
                0xa000..=0xbfff if state.map_select <= 0x03 => {
                    let offset = 0x2000 * (state.map_select & 0b11) as usize;
                    self.write_ram(offset, address, value);
                }
                _ => {}
            },
        }

        Ok(())
    }
}
