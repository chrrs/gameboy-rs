use bitflags::bitflags;

use crate::{
    instruction::{CpuFlag, CpuRegister, Instruction, InstructionOperand},
    mmu::Mmu,
};

bitflags! {
    struct CpuFlags: u8 {
        const ZERO = 1 << 7;
        const SUBTRACTION = 1 << 6;
        const HALF_CARRY = 1 << 5;
        const CARRY = 1 << 4;
    }
}
pub struct Cpu {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    flags: CpuFlags,
    sp: u16,
    pc: u16,
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            flags: CpuFlags::empty(),
            sp: 0,
            pc: 0,
        }
    }

    pub fn af(&self) -> u16 {
        (self.a as u16) << 8 | (self.flags.bits() as u16)
    }

    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.flags = unsafe { CpuFlags::from_bits_unchecked(value as u8) };
    }

    pub fn bc(&self) -> u16 {
        (self.b as u16) << 8 | (self.c as u16)
    }

    pub fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = value as u8;
    }

    pub fn de(&self) -> u16 {
        (self.d as u16) << 8 | (self.e as u16)
    }

    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = value as u8;
    }

    pub fn hl(&self) -> u16 {
        (self.h as u16) << 8 | (self.l as u16)
    }

    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = value as u8;
    }

    pub fn fetch_instruction(&mut self, mmu: &mut Mmu) -> Option<Instruction> {
        let opcode = self.fetch_u8(mmu);

        match opcode {
            0x00 => Some(Instruction::Noop),
            0x02 => Some(Instruction::Load(
                InstructionOperand::MemoryLocationRegister(CpuRegister::BC),
                InstructionOperand::Register(CpuRegister::A),
            )),
            0x04 => Some(Instruction::Increment(InstructionOperand::Register(
                CpuRegister::B,
            ))),
            0x05 => Some(Instruction::Decrement(InstructionOperand::Register(
                CpuRegister::B,
            ))),
            0x06 => Some(Instruction::Load(
                InstructionOperand::Register(CpuRegister::B),
                InstructionOperand::Immediate8(self.fetch_u8(mmu)),
            )),
            0x0c => Some(Instruction::Increment(InstructionOperand::Register(
                CpuRegister::C,
            ))),
            0x0d => Some(Instruction::Decrement(InstructionOperand::Register(
                CpuRegister::C,
            ))),
            0x0e => Some(Instruction::Load(
                InstructionOperand::Register(CpuRegister::C),
                InstructionOperand::Immediate8(self.fetch_u8(mmu)),
            )),
            0x11 => Some(Instruction::Load(
                InstructionOperand::Register(CpuRegister::DE),
                InstructionOperand::Immediate16(self.fetch_u16(mmu)),
            )),
            0x13 => Some(Instruction::Increment(InstructionOperand::Register(
                CpuRegister::DE,
            ))),
            0x15 => Some(Instruction::Decrement(InstructionOperand::Register(
                CpuRegister::D,
            ))),
            0x16 => Some(Instruction::Load(
                InstructionOperand::Register(CpuRegister::D),
                InstructionOperand::Immediate8(self.fetch_u8(mmu)),
            )),
            0x17 => Some(Instruction::RotateLeft(InstructionOperand::Register(
                CpuRegister::A,
            ))),
            0x18 => Some(Instruction::JumpRelative(self.fetch_u8(mmu) as i8)),
            0x1a => Some(Instruction::Load(
                InstructionOperand::Register(CpuRegister::A),
                InstructionOperand::MemoryLocationRegister(CpuRegister::DE),
            )),
            0x1d => Some(Instruction::Decrement(InstructionOperand::Register(
                CpuRegister::E,
            ))),
            0x1e => Some(Instruction::Load(
                InstructionOperand::Register(CpuRegister::E),
                InstructionOperand::Immediate8(self.fetch_u8(mmu)),
            )),
            0x20 => Some(Instruction::JumpRelativeIf(
                CpuFlag::Zero,
                false,
                self.fetch_u8(mmu) as i8,
            )),
            0x21 => Some(Instruction::Load(
                InstructionOperand::Register(CpuRegister::HL),
                InstructionOperand::Immediate16(self.fetch_u16(mmu)),
            )),
            0x22 => Some(Instruction::LoadMemInc(
                CpuRegister::HL,
                InstructionOperand::Register(CpuRegister::A),
            )),
            0x23 => Some(Instruction::Increment(InstructionOperand::Register(
                CpuRegister::HL,
            ))),
            0x24 => Some(Instruction::Increment(InstructionOperand::Register(
                CpuRegister::H,
            ))),
            0x28 => Some(Instruction::JumpRelativeIf(
                CpuFlag::Zero,
                true,
                self.fetch_u8(mmu) as i8,
            )),
            0x2e => Some(Instruction::Load(
                InstructionOperand::Register(CpuRegister::L),
                InstructionOperand::Immediate8(self.fetch_u8(mmu)),
            )),
            0x31 => Some(Instruction::Load(
                InstructionOperand::Register(CpuRegister::SP),
                InstructionOperand::Immediate16(self.fetch_u16(mmu)),
            )),
            0x32 => Some(Instruction::LoadMemDec(
                CpuRegister::HL,
                InstructionOperand::Register(CpuRegister::A),
            )),
            0x3d => Some(Instruction::Decrement(InstructionOperand::Register(
                CpuRegister::A,
            ))),
            0x3e => Some(Instruction::Load(
                InstructionOperand::Register(CpuRegister::A),
                InstructionOperand::Immediate8(self.fetch_u8(mmu)),
            )),
            0x4f => Some(Instruction::Load(
                InstructionOperand::Register(CpuRegister::C),
                InstructionOperand::Register(CpuRegister::A),
            )),
            0x57 => Some(Instruction::Load(
                InstructionOperand::Register(CpuRegister::D),
                InstructionOperand::Register(CpuRegister::A),
            )),
            0x67 => Some(Instruction::Load(
                InstructionOperand::Register(CpuRegister::H),
                InstructionOperand::Register(CpuRegister::A),
            )),
            0x77 => Some(Instruction::Load(
                InstructionOperand::MemoryLocationRegister(CpuRegister::HL),
                InstructionOperand::Register(CpuRegister::A),
            )),
            0x78 => Some(Instruction::Load(
                InstructionOperand::Register(CpuRegister::A),
                InstructionOperand::Register(CpuRegister::B),
            )),
            0x7b => Some(Instruction::Load(
                InstructionOperand::Register(CpuRegister::A),
                InstructionOperand::Register(CpuRegister::E),
            )),
            0x7c => Some(Instruction::Load(
                InstructionOperand::Register(CpuRegister::A),
                InstructionOperand::Register(CpuRegister::H),
            )),
            0x7d => Some(Instruction::Load(
                InstructionOperand::Register(CpuRegister::A),
                InstructionOperand::Register(CpuRegister::L),
            )),
            0x86 => Some(Instruction::Add(
                CpuRegister::A,
                InstructionOperand::MemoryLocationRegister(CpuRegister::HL),
            )),
            0x90 => Some(Instruction::Subtract(InstructionOperand::Register(
                CpuRegister::B,
            ))),
            0xaf => Some(Instruction::Xor(InstructionOperand::Register(
                CpuRegister::A,
            ))),
            0xbe => Some(Instruction::Compare(
                InstructionOperand::MemoryLocationRegister(CpuRegister::HL),
            )),
            0xc1 => Some(Instruction::Pop(CpuRegister::BC)),
            0xc5 => Some(Instruction::Push(CpuRegister::BC)),
            0xc9 => Some(Instruction::Return),
            0xcb => self.fetch_extended_instruction(mmu),
            0xcd => Some(Instruction::Call(self.fetch_u16(mmu))),
            0xe0 => Some(Instruction::Load(
                InstructionOperand::MemoryLocationImmediate16(0xff00 + self.fetch_u8(mmu) as u16),
                InstructionOperand::Register(CpuRegister::A),
            )),
            0xe2 => Some(Instruction::Load(
                InstructionOperand::OffsetMemoryLocationRegister(0xff00, CpuRegister::C),
                InstructionOperand::Register(CpuRegister::A),
            )),
            0xea => Some(Instruction::Load(
                InstructionOperand::MemoryLocationImmediate16(self.fetch_u16(mmu)),
                InstructionOperand::Register(CpuRegister::A),
            )),
            0xf0 => Some(Instruction::Load(
                InstructionOperand::Register(CpuRegister::A),
                InstructionOperand::MemoryLocationImmediate16(0xff00 + self.fetch_u8(mmu) as u16),
            )),
            0xfe => Some(Instruction::Compare(InstructionOperand::Immediate8(
                self.fetch_u8(mmu),
            ))),
            _ => {
                println!("Unknown instruction for opcode {:#04x}", opcode);
                None
            }
        }
    }

    fn fetch_extended_instruction(&mut self, mmu: &mut Mmu) -> Option<Instruction> {
        let opcode = self.fetch_u8(mmu);

        match opcode {
            0x11 => Some(Instruction::RotateLeft(InstructionOperand::Register(
                CpuRegister::C,
            ))),
            0x7c => Some(Instruction::Bit(
                7,
                InstructionOperand::Register(CpuRegister::H),
            )),
            _ => {
                println!("Unknown instruction for opcode 0xcb {:#04x}", opcode);
                None
            }
        }
    }

    fn fetch_u8(&mut self, mmu: &mut Mmu) -> u8 {
        let ret = mmu.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        ret
    }

    fn fetch_u16(&mut self, mmu: &mut Mmu) -> u16 {
        let ret = (mmu.read(self.pc + 1) as u16) << 8 | (mmu.read(self.pc) as u16);
        self.pc = self.pc.wrapping_add(2);
        ret
    }
}
