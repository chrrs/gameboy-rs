use std::fs::File;

use gameboy::{bios::DMG_BIOS, cartridge::Cartridge, cpu::Cpu, mmu::Mmu};

fn main() {
    let cart = Cartridge::new(File::open("./roms/blargg/cpu_instrs.gb").unwrap()).unwrap();
    println!("Valid cart? {}", cart.verify());
    println!("Cart title: {}", cart.title().unwrap_or("<unknown>"));

    let mut mmu = Mmu::new(DMG_BIOS, cart);
    let mut cpu = Cpu::new();

    while cpu.pc <= 0x100 {
        let instruction = cpu.fetch_instruction(&mut mmu).unwrap();
        println!(
            "A:{:#x} B:{:#x} C:{:#x} D:{:#x} E:{:#x} H:{:#x} L:{:#x} F:{:#x}, Executing {:x?}",
            cpu.a, cpu.b, cpu.c, cpu.d, cpu.e, cpu.h, cpu.l, cpu.f, instruction
        );
        cpu.exec_instruction(&mut mmu, instruction);
    }
}
