use std::fs::File;

use gameboy::{
    bios::DMG_BIOS,
    cartridge::Cartridge,
    cpu::{Cpu, CpuFlag},
    mmu::Mmu,
};

fn main() {
    let cart = Cartridge::new(File::open("./roms/blargg/cpu_instrs.gb").unwrap()).unwrap();
    println!("Valid cart? {}", cart.verify());
    println!("Cart title: {}", cart.title().unwrap_or("<unknown>"));

    let mut mmu = Mmu::new(DMG_BIOS, cart);
    let mut cpu = Cpu::new();

    while cpu.pc <= 0x100 {
        let pc = cpu.pc;
        let instruction = cpu.fetch_instruction(&mut mmu).unwrap();
        println!(
            "A:{:#04x} B:{:#04x} C:{:#04x} D:{:#04x} E:{:#04x} H:{:#04x} L:{:#04x} F:{}{}{}{} SP:{:#06x} PC:{:#06x}, Executing {:x?}",
            cpu.a, cpu.b, cpu.c, cpu.d, cpu.e, cpu.h, cpu.l, 
            if cpu.get_flag(CpuFlag::Zero) {"Z"} else {"-"}, 
            if cpu.get_flag(CpuFlag::Subtraction) {"S"} else {"-"}, 
            if cpu.get_flag(CpuFlag::HalfCarry) {"H"} else {"-"}, 
            if cpu.get_flag(CpuFlag::Carry) {"C"} else {"-"}, 
            cpu.sp, pc, instruction
        );
        cpu.exec_instruction(&mut mmu, instruction);
    }
}
