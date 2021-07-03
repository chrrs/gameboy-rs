use std::fmt;

use crate::cpu::CpuFlag;

#[derive(Debug, Clone, Copy)]
pub enum CpuRegister {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    F,
    AF,
    BC,
    DE,
    HL,
    SP,
}

impl CpuRegister {
    pub fn is_16bit(&self) -> bool {
        match self {
            CpuRegister::A => false,
            CpuRegister::B => false,
            CpuRegister::C => false,
            CpuRegister::D => false,
            CpuRegister::E => false,
            CpuRegister::H => false,
            CpuRegister::L => false,
            CpuRegister::F => false,
            CpuRegister::AF => true,
            CpuRegister::BC => true,
            CpuRegister::DE => true,
            CpuRegister::HL => true,
            CpuRegister::SP => true,
        }
    }
}

impl fmt::Display for CpuRegister {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CpuRegister::A => write!(f, "A"),
            CpuRegister::B => write!(f, "B"),
            CpuRegister::C => write!(f, "C"),
            CpuRegister::D => write!(f, "D"),
            CpuRegister::E => write!(f, "E"),
            CpuRegister::H => write!(f, "H"),
            CpuRegister::L => write!(f, "L"),
            CpuRegister::F => write!(f, "F"),
            CpuRegister::AF => write!(f, "AF"),
            CpuRegister::BC => write!(f, "BC"),
            CpuRegister::DE => write!(f, "DE"),
            CpuRegister::HL => write!(f, "HL"),
            CpuRegister::SP => write!(f, "SP"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum InstructionOperand {
    Register(CpuRegister),
    Immediate8(u8),
    Immediate16(u16),
    OffsetMemoryLocationRegister(u16, CpuRegister),
    MemoryLocationRegister(CpuRegister),
    MemoryLocationRegisterDecrement(CpuRegister),
    MemoryLocationRegisterIncrement(CpuRegister),
    MemoryLocationImmediate16(u16),
}

impl InstructionOperand {
    pub fn is_16bit(&self) -> bool {
        match self {
            InstructionOperand::Register(reg) => reg.is_16bit(),
            InstructionOperand::Immediate8(_) => false,
            InstructionOperand::Immediate16(_) => true,
            InstructionOperand::OffsetMemoryLocationRegister(_, _) => false,
            InstructionOperand::MemoryLocationRegister(_) => false,
            InstructionOperand::MemoryLocationRegisterDecrement(_) => false,
            InstructionOperand::MemoryLocationRegisterIncrement(_) => false,
            InstructionOperand::MemoryLocationImmediate16(_) => false,
        }
    }

    pub fn cycles(&self, affect_16bit_reg: bool) -> usize {
        match self {
            InstructionOperand::Register(reg) => {
                if affect_16bit_reg && reg.is_16bit() {
                    1
                } else {
                    0
                }
            }
            InstructionOperand::Immediate8(_) => 1,
            InstructionOperand::Immediate16(_) => 2,
            InstructionOperand::OffsetMemoryLocationRegister(_, _) => 2,
            InstructionOperand::MemoryLocationRegister(reg) => {
                if affect_16bit_reg && reg.is_16bit() {
                    2
                } else {
                    1
                }
            }
            InstructionOperand::MemoryLocationRegisterDecrement(reg) => {
                if affect_16bit_reg && reg.is_16bit() {
                    2
                } else {
                    1
                }
            }
            InstructionOperand::MemoryLocationRegisterIncrement(reg) => {
                if affect_16bit_reg && reg.is_16bit() {
                    2
                } else {
                    1
                }
            }
            InstructionOperand::MemoryLocationImmediate16(_) => 3,
        }
    }
}

impl fmt::Display for InstructionOperand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstructionOperand::Register(reg) => reg.fmt(f),
            InstructionOperand::Immediate8(value) => write!(f, "{:#04x}", value),
            InstructionOperand::Immediate16(value) => write!(f, "{:#06x}", value),
            InstructionOperand::OffsetMemoryLocationRegister(offset, reg) => {
                write!(f, "({:#06x}+{})", offset, reg)
            }
            InstructionOperand::MemoryLocationRegister(reg) => write!(f, "({})", reg),
            InstructionOperand::MemoryLocationRegisterDecrement(reg) => write!(f, "({}-)", reg),
            InstructionOperand::MemoryLocationRegisterIncrement(reg) => write!(f, "({}+)", reg),
            InstructionOperand::MemoryLocationImmediate16(address) => {
                write!(f, "({:#06x})", address)
            }
        }
    }
}

#[derive(Debug)]
pub enum Instruction {
    Noop,
    Stop,
    Load(InstructionOperand, InstructionOperand),
    Xor(InstructionOperand),
    Bit(u8, InstructionOperand),
    JumpRelative(i8),
    JumpRelativeIf(CpuFlag, bool, i8),
    Increment(InstructionOperand),
    Decrement(InstructionOperand),
    Call(u16),
    Compare(InstructionOperand),
    Add(CpuRegister, InstructionOperand),
    Subtract(InstructionOperand),
    Push(CpuRegister),
    Pop(CpuRegister),
    RotateLeftA,
    ExtendedRotateLeft(InstructionOperand),
    Return,
}

impl Instruction {
    pub fn cycles(&self) -> usize {
        match self {
            Instruction::Noop => 1,
            Instruction::Stop => 1,
            Instruction::Load(to, from) => {
                1 + to.cycles(false) + from.cycles(false) + if to.is_16bit() { 1 } else { 0 }
            }
            Instruction::Xor(from) => 1 + from.cycles(false),
            Instruction::Bit(_, from) => 2 + from.cycles(false),
            Instruction::JumpRelative(_) => 3,
            Instruction::JumpRelativeIf(_, _, _) => 2,
            Instruction::Increment(to) => 1 + to.cycles(true),
            Instruction::Decrement(to) => 1 + to.cycles(true),
            Instruction::Call(_) => 6,
            Instruction::Compare(to) => 1 + to.cycles(false),
            Instruction::Add(to, from) => {
                1 + from.cycles(false) + if to.is_16bit() { 1 } else { 0 }
            }
            Instruction::Subtract(from) => 1 + from.cycles(false),
            Instruction::Push(_) => 4,
            Instruction::Pop(_) => 3,
            Instruction::RotateLeftA => 1,
            Instruction::ExtendedRotateLeft(to) => 2 + to.cycles(true),
            Instruction::Return => 4,
        }
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::Noop => write!(f, "noop"),
            Instruction::Stop => write!(f, "stop"),
            Instruction::Load(to, from) => write!(f, "ld {}, {}", to, from),
            Instruction::Xor(from) => write!(f, "xor {}", from),
            Instruction::Bit(bit, from) => write!(f, "bit {}, {}", bit, from),
            Instruction::JumpRelative(offset) => write!(f, "jr {}", offset),
            Instruction::JumpRelativeIf(flag, expected, offset) => {
                write!(
                    f,
                    "jr {}{}, {}",
                    if *expected { "" } else { "N" },
                    flag,
                    offset
                )
            }
            Instruction::Increment(to) => write!(f, "inc {}", to),
            Instruction::Decrement(to) => write!(f, "dec {}", to),
            Instruction::Call(address) => write!(f, "call {:#06x}", address),
            Instruction::Compare(from) => write!(f, "cp {}", from),
            Instruction::Add(to, from) => write!(f, "add {}, {}", to, from),
            Instruction::Subtract(from) => write!(f, "sub {}", from),
            Instruction::Push(from) => write!(f, "push {}", from),
            Instruction::Pop(from) => write!(f, "pop {}", from),
            Instruction::RotateLeftA => write!(f, "rla"),
            Instruction::ExtendedRotateLeft(to) => write!(f, "rl {}", to),
            Instruction::Return => write!(f, "ret"),
        }
    }
}
