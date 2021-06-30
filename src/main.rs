use std::fs::File;

use gameboy::{bios::DMG_BIOS, cartridge::Cartridge, cpu::Cpu, mmu::Mmu};

fn main() {
    let cart = Cartridge::new(File::open("./roms/Tetris.gb").unwrap()).unwrap();
    println!("Valid cart? {}", cart.verify());
    println!("Cart title: {}", cart.title().unwrap_or("<unknown>"));

    let mut mmu = Mmu::new(DMG_BIOS, cart);
    let mut cpu = Cpu::new();

    while let Some(instruction) = cpu.fetch_instruction(&mut mmu) {
        println!("{:x?}", instruction)
    }
}
