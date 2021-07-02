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

    pub fn cycles(&self) -> usize {
        match self {
            InstructionOperand::Register(_) => 0,
            InstructionOperand::Immediate8(_) => 1,
            InstructionOperand::Immediate16(_) => 2,
            InstructionOperand::OffsetMemoryLocationRegister(_, _) => 2,
            InstructionOperand::MemoryLocationRegister(_) => 1,
            InstructionOperand::MemoryLocationRegisterDecrement(_) => 1,
            InstructionOperand::MemoryLocationRegisterIncrement(_) => 1,
            InstructionOperand::MemoryLocationImmediate16(_) => 3,
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
    RotateLeft(InstructionOperand),
    Return,
}

impl Instruction {
    pub fn cycles(&self) -> usize {
        match self {
            Instruction::Noop => 1,
            Instruction::Stop => 1,
            Instruction::Load(to, from) => {
                1 + to.cycles() + from.cycles() + if to.is_16bit() { 1 } else { 0 }
            }
            Instruction::Xor(from) => 1 + from.cycles(),
            Instruction::Bit(_, from) => 2 + from.cycles(),
            Instruction::JumpRelative(_) => 2,
            Instruction::JumpRelativeIf(_, _, _) => 2,
            Instruction::Increment(to) => {
                1 + match to {
                    InstructionOperand::Register(reg) => {
                        if reg.is_16bit() {
                            1
                        } else {
                            0
                        }
                    }
                    InstructionOperand::OffsetMemoryLocationRegister(_, _) => 1,
                    InstructionOperand::MemoryLocationRegister(_) => 2,
                    InstructionOperand::MemoryLocationRegisterDecrement(_) => 2,
                    InstructionOperand::MemoryLocationRegisterIncrement(_) => 2,
                    InstructionOperand::MemoryLocationImmediate16(_) => 2,
                    _ => 0,
                }
            }
            Instruction::Decrement(to) => {
                1 + match to {
                    InstructionOperand::Register(reg) => {
                        if reg.is_16bit() {
                            1
                        } else {
                            0
                        }
                    }
                    InstructionOperand::OffsetMemoryLocationRegister(_, _) => 1,
                    InstructionOperand::MemoryLocationRegister(_) => 2,
                    InstructionOperand::MemoryLocationRegisterDecrement(_) => 2,
                    InstructionOperand::MemoryLocationRegisterIncrement(_) => 2,
                    InstructionOperand::MemoryLocationImmediate16(_) => 2,
                    _ => 0,
                }
            }
            Instruction::Call(_) => todo!(),
            Instruction::Compare(_) => todo!(),
            Instruction::Add(_, _) => todo!(),
            Instruction::Subtract(_) => todo!(),
            Instruction::Push(_) => todo!(),
            Instruction::Pop(_) => todo!(),
            Instruction::RotateLeft(_) => todo!(),
            Instruction::Return => todo!(),
        }
    }
}
