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
    OffsetMemoryLocationImmediate8(u16, u8),
    MemoryLocationImmediate16(u16),
    DoubleMemoryLocationImmediate16(u16),
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
            InstructionOperand::OffsetMemoryLocationImmediate8(_, _) => false,
            InstructionOperand::MemoryLocationImmediate16(_) => false,
            InstructionOperand::DoubleMemoryLocationImmediate16(_) => true,
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
            InstructionOperand::OffsetMemoryLocationRegister(_, _) => 1,
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
            InstructionOperand::OffsetMemoryLocationImmediate8(_, _) => 2,
            InstructionOperand::MemoryLocationImmediate16(_) => 3,
            InstructionOperand::DoubleMemoryLocationImmediate16(_) => 4,
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
            InstructionOperand::OffsetMemoryLocationImmediate8(offset, address) => {
                write!(f, "({:#06x}+{:#04x})", offset, address)
            }
            InstructionOperand::MemoryLocationImmediate16(address) => {
                write!(f, "({:#06x})", address)
            }
            InstructionOperand::DoubleMemoryLocationImmediate16(address) => {
                write!(f, "({:#06x})", address)
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SPOps {
    AddOffset(i8),
    LoadIntoHL(i8),
    LoadFromHL,
}

impl SPOps {
    pub fn cycles(&self) -> usize {
        match self {
            SPOps::AddOffset(_) => 4,
            SPOps::LoadIntoHL(_) => 3,
            SPOps::LoadFromHL => 2,
        }
    }
}

impl fmt::Display for SPOps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SPOps::AddOffset(offset) => write!(f, "add SP, {}", offset),
            SPOps::LoadIntoHL(offset) => write!(f, "ld HL, SP{:+}", offset),
            SPOps::LoadFromHL => write!(f, "ld SP, HL"),
        }
    }
}

#[derive(Debug)]
pub enum Instruction {
    Noop,
    Stop,
    Load(InstructionOperand, InstructionOperand),
    And(InstructionOperand),
    Or(InstructionOperand),
    Xor(InstructionOperand),
    Bit(u8, InstructionOperand),
    Jump(InstructionOperand),
    JumpIf(CpuFlag, bool, u16),
    JumpRelative(i8),
    JumpRelativeIf(CpuFlag, bool, i8),
    Increment(InstructionOperand),
    Decrement(InstructionOperand),
    Call(u16),
    CallIf(CpuFlag, bool, u16),
    Compare(InstructionOperand),
    Add8(CpuRegister, InstructionOperand, bool),
    Add16(CpuRegister, InstructionOperand),
    Subtract(InstructionOperand, bool),
    Push(CpuRegister),
    Pop(CpuRegister),
    RotateLeftA(bool),
    RotateLeft(InstructionOperand, bool),
    RotateRightA(bool),
    RotateRight(InstructionOperand, bool),
    ShiftLeft(InstructionOperand),
    ShiftRight(InstructionOperand, bool),
    Return,
    ReturnIf(CpuFlag, bool),
    ReturnInterrupt,
    DisableInterrupts,
    EnableInterrupts,
    Complement,
    Swap(InstructionOperand),
    Rst(u8),
    DAA,
    SetBit(u8, InstructionOperand, bool),
    SPOps(SPOps),
    SetCarryFlag(bool),
    Halt,
}

impl Instruction {
    pub fn cycles(&self) -> usize {
        match self {
            Instruction::Noop => 1,
            Instruction::Stop => 0,
            Instruction::Load(to, from) => 1 + to.cycles(false) + from.cycles(false),
            Instruction::And(from) => 1 + from.cycles(false),
            Instruction::Or(from) => 1 + from.cycles(false),
            Instruction::Xor(from) => 1 + from.cycles(false),
            Instruction::Bit(_, from) => 2 + from.cycles(false),
            Instruction::Jump(to) => {
                if let InstructionOperand::Register(_) = to {
                    1
                } else {
                    4
                }
            }
            Instruction::JumpIf(_, _, _) => 3,
            Instruction::JumpRelative(_) => 3,
            Instruction::JumpRelativeIf(_, _, _) => 2,
            Instruction::Increment(to) => 1 + to.cycles(true),
            Instruction::Decrement(to) => 1 + to.cycles(true),
            Instruction::Call(_) => 6,
            Instruction::CallIf(_, _, _) => 3,
            Instruction::Compare(to) => 1 + to.cycles(false),
            Instruction::Add8(_, from, _) => 1 + from.cycles(false),
            Instruction::Add16(_, from) => 2 + from.cycles(false),
            Instruction::Subtract(from, _) => 1 + from.cycles(false),
            Instruction::Push(_) => 4,
            Instruction::Pop(_) => 3,
            Instruction::RotateLeftA(_) => 1,
            Instruction::RotateLeft(to, _) => 2 + to.cycles(true),
            Instruction::RotateRightA(_) => 1,
            Instruction::RotateRight(to, _) => 2 + to.cycles(true),
            Instruction::ShiftRight(to, _) => 2 + to.cycles(true),
            Instruction::ShiftLeft(to) => 2 + to.cycles(true),
            Instruction::Return => 4,
            Instruction::ReturnIf(_, _) => 2,
            Instruction::ReturnInterrupt => 4,
            Instruction::DisableInterrupts => 1,
            Instruction::EnableInterrupts => 1,
            Instruction::Complement => 1,
            Instruction::Swap(to) => 2 + to.cycles(true),
            Instruction::Rst(_) => 4,
            Instruction::DAA => 1,
            Instruction::SetBit(_, to, _) => 2 + to.cycles(true),
            Instruction::SPOps(op) => op.cycles(),
            Instruction::SetCarryFlag(_) => 1,
            Instruction::Halt => 1,
        }
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::Noop => write!(f, "noop"),
            Instruction::Stop => write!(f, "stop"),
            Instruction::Load(to, from) => write!(f, "ld {}, {}", to, from),
            Instruction::And(from) => write!(f, "and {}", from),
            Instruction::Or(from) => write!(f, "or {}", from),
            Instruction::Xor(from) => write!(f, "xor {}", from),
            Instruction::Bit(bit, from) => write!(f, "bit {}, {}", bit, from),
            Instruction::Jump(to) => write!(f, "jp {}", to),
            Instruction::JumpIf(flag, expected, to) => {
                write!(f, "jp {}{}, {}", if *expected { "" } else { "N" }, flag, to)
            }
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
            Instruction::CallIf(flag, expected, address) => {
                write!(
                    f,
                    "call {}{}, {}",
                    if *expected { "" } else { "N" },
                    flag,
                    address
                )
            }
            Instruction::Compare(from) => write!(f, "cp {}", from),
            Instruction::Add8(to, from, use_carry) => write!(
                f,
                "{} {}, {}",
                if *use_carry { "adc" } else { "add" },
                to,
                from
            ),
            Instruction::Add16(to, from) => write!(f, "add {}, {}", to, from),
            Instruction::Subtract(from, use_carry) => {
                write!(f, "{} {}", if *use_carry { "sbc" } else { "sub" }, from)
            }
            Instruction::Push(from) => write!(f, "push {}", from),
            Instruction::Pop(from) => write!(f, "pop {}", from),
            Instruction::RotateLeftA(use_carry) => {
                write!(f, "rl{}a", if *use_carry { "c" } else { "" })
            }
            Instruction::RotateLeft(to, use_carry) => {
                write!(f, "rl{} {}", if *use_carry { "c" } else { "" }, to)
            }
            Instruction::RotateRightA(use_carry) => {
                write!(f, "rr{}a", if *use_carry { "c" } else { "" })
            }
            Instruction::RotateRight(to, use_carry) => {
                write!(f, "rr{} {}", if *use_carry { "c" } else { "" }, to)
            }
            Instruction::ShiftRight(to, zero) => {
                write!(f, "sr{} {}", if *zero { "l" } else { "a" }, to)
            }
            Instruction::ShiftLeft(to) => {
                write!(f, "sla {}", to)
            }
            Instruction::Return => write!(f, "ret"),
            Instruction::ReturnIf(flag, expected) => {
                write!(f, "ret {}{}", if *expected { "" } else { "N" }, flag)
            }
            Instruction::ReturnInterrupt => write!(f, "reti"),
            Instruction::DisableInterrupts => write!(f, "di"),
            Instruction::EnableInterrupts => write!(f, "ei"),
            Instruction::Complement => write!(f, "cpl"),
            Instruction::Swap(to) => write!(f, "swap {}", to),
            Instruction::Rst(address) => write!(f, "rst {}", address),
            Instruction::DAA => write!(f, "daa"),
            Instruction::SetBit(bit, to, set) => {
                write!(f, "{} {}, {}", if *set { "set" } else { "res" }, bit, to)
            }
            Instruction::SPOps(op) => op.fmt(f),
            Instruction::SetCarryFlag(toggle) => write!(f, "{}cf", if *toggle { "c" } else { "s" }),
            Instruction::Halt => write!(f, "halt"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU16, Ordering};

    use crate::{
        cpu::Cpu,
        memory::{Memory, MemoryError},
    };

    const OPCODE_CYCLES: [usize; 256] = [
        1, 3, 2, 2, 1, 1, 2, 1, 5, 2, 2, 2, 1, 1, 2, 1, 0, 3, 2, 2, 1, 1, 2, 1, 3, 2, 2, 2, 1, 1,
        2, 1, 2, 3, 2, 2, 1, 1, 2, 1, 2, 2, 2, 2, 1, 1, 2, 1, 2, 3, 2, 2, 3, 3, 3, 1, 2, 2, 2, 2,
        1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1,
        1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 2, 2, 2, 2, 2, 2, 1, 2,
        1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1,
        2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1,
        1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 2, 3, 3, 4, 3, 4, 2, 4, 2, 4, 3, 0, 3, 6, 2, 4, 2, 3,
        3, 0, 3, 4, 2, 4, 2, 4, 3, 0, 3, 0, 2, 4, 3, 3, 2, 0, 0, 4, 2, 4, 4, 1, 4, 0, 0, 0, 2, 4,
        3, 3, 2, 1, 0, 4, 2, 4, 3, 2, 4, 1, 0, 0, 2, 4,
    ];

    const EXTENDED_OPCODE_CYCLES: [usize; 256] = [
        2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2,
        4, 2, 2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2,
        2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 3, 2, 2, 2, 2, 2, 2, 2, 3, 2, 2, 2, 2, 2, 2, 2, 3, 2, 2, 2,
        2, 2, 2, 2, 3, 2, 2, 2, 2, 2, 2, 2, 3, 2, 2, 2, 2, 2, 2, 2, 3, 2, 2, 2, 2, 2, 2, 2, 3, 2,
        2, 2, 2, 2, 2, 2, 3, 2, 2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2,
        4, 2, 2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2,
        2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, 2, 2,
        2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2,
        2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2,
    ];

    struct InstructionMemory(pub AtomicU16);

    impl Memory for InstructionMemory {
        fn read(&self, _address: u16) -> Result<u8, MemoryError> {
            Ok(self
                .0
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| Some(x >> 8))
                .unwrap() as u8)
        }

        fn write(&mut self, _address: u16, _value: u8) -> Result<(), MemoryError> {
            unreachable!()
        }
    }

    #[test]
    fn instruction_cycles() {
        let mut memory = InstructionMemory(AtomicU16::new(0));
        let mut cpu = Cpu::new();

        for opcode in 0..=0xff {
            memory.0.store(opcode, Ordering::SeqCst);
            let instruction = cpu.fetch_instruction(&mut memory);

            if opcode == 0xcb {
                continue;
            }

            if let Ok(instruction) = instruction {
                assert_eq!(
                    instruction.cycles(),
                    OPCODE_CYCLES[opcode as usize],
                    "incorrect cycle count for opcode {:#04x} ({})",
                    opcode,
                    instruction
                )
            }
        }
    }

    #[test]
    fn extended_instruction_cycles() {
        let mut memory = InstructionMemory(AtomicU16::new(0));
        let mut cpu = Cpu::new();

        for opcode in 0x00..=0xff {
            let opcode = opcode << 8 | 0xcb;
            memory.0.store(opcode, Ordering::SeqCst);
            let instruction = cpu.fetch_instruction(&mut memory);

            if let Ok(instruction) = instruction {
                assert_eq!(
                    instruction.cycles(),
                    EXTENDED_OPCODE_CYCLES[opcode as usize >> 8],
                    "incorrect cycle count for opcode {:#06x} ({})",
                    opcode,
                    instruction
                )
            }
        }
    }
}
