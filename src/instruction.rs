#[derive(Debug)]
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

#[derive(Debug)]
pub enum CpuFlag {
    Zero,
    Subtraction,
    HalfCarry,
    Carry,
}

#[derive(Debug)]
pub enum InstructionOperand {
    Register(CpuRegister),
    Immediate8(u8),
    Immediate16(u16),
    OffsetMemoryLocationRegister(u16, CpuRegister),
    OffsetMemoryLocationImmediate8(u16, u8),
    MemoryLocationRegister(CpuRegister),
    MemoryLocationImmediate16(u16),
}

#[derive(Debug)]
pub enum Instruction {
    Noop,
    Stop,
    Load(InstructionOperand, InstructionOperand),
    LoadMemDec(CpuRegister, InstructionOperand),
    LoadMemInc(CpuRegister, InstructionOperand),
    Xor(InstructionOperand),
    Bit(u8, InstructionOperand),
    JumpRelativeIf(CpuFlag, bool, i8),
    Increment(InstructionOperand),
    Decrement(InstructionOperand),
    Call(u16),
    Compare(InstructionOperand),
}
