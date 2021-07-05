use std::{collections::BTreeMap, fmt, u8};
use thiserror::Error;

use crate::{
    instruction::{CpuRegister, Instruction, InstructionOperand},
    memory::{Memory, MemoryError, MemoryOperation},
};

#[derive(Error, Debug, Clone, Copy)]
pub enum InstructionError {
    #[error("invalid opcode {opcode:#04x}")]
    InvalidOpcode { opcode: u16 },
    #[error("memory error")]
    MemoryError(#[from] MemoryError),
}

#[derive(Error, Debug, Clone, Copy)]
pub enum CpuError {
    #[error("{op} to {operand} with mismatched argument size")]
    OperandSizeMismatch {
        operand: InstructionOperand,
        op: MemoryOperation,
    },
    #[error("access to immediate operand with mismatched argument size")]
    ImmediateSizeMismatch,
    #[error("write to immediate operand")]
    ImmediateWrite,
    #[error("memory error")]
    MemoryError(#[from] MemoryError),
    #[error("instruction error")]
    InstructionError(#[from] InstructionError),
}

#[derive(Debug, Clone, Copy)]
pub enum CpuFlag {
    Zero,
    Subtraction,
    HalfCarry,
    Carry,
}

impl CpuFlag {
    pub fn bit(&self) -> u8 {
        match self {
            CpuFlag::Zero => 1 << 7,
            CpuFlag::Subtraction => 1 << 6,
            CpuFlag::HalfCarry => 1 << 5,
            CpuFlag::Carry => 1 << 4,
        }
    }
}

impl fmt::Display for CpuFlag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CpuFlag::Zero => write!(f, "Z"),
            CpuFlag::Subtraction => write!(f, "S"),
            CpuFlag::HalfCarry => write!(f, "H"),
            CpuFlag::Carry => write!(f, "C"),
        }
    }
}

pub struct Cpu {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub f: u8,
    pub sp: u16,
    pub pc: u16,
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
            f: 0,
            sp: 0,
            pc: 0,
        }
    }

    pub fn reset(&mut self) {
        self.a = 0;
        self.b = 0;
        self.c = 0;
        self.d = 0;
        self.e = 0;
        self.h = 0;
        self.l = 0;
        self.f = 0;
        self.pc = 0;
    }

    pub fn af(&self) -> u16 {
        (self.a as u16) << 8 | (self.f as u16)
    }

    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = value as u8;
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

    pub fn get_flag(&self, flag: CpuFlag) -> bool {
        self.f & flag.bit() != 0
    }

    pub fn set_flag(&mut self, flag: CpuFlag, value: bool) {
        if value {
            self.f |= flag.bit()
        } else {
            self.f &= !flag.bit()
        }
    }
}

impl Cpu {
    pub fn pop_u16<M: Memory>(&mut self, mem: &mut M) -> Result<u16, MemoryError> {
        let lo = mem.read(self.sp)?;
        self.sp = self.sp.wrapping_add(1);
        let hi = mem.read(self.sp)?;
        self.sp = self.sp.wrapping_add(1);

        Ok((hi as u16) << 8 | (lo as u16))
    }

    pub fn push_u16<M: Memory>(&mut self, mem: &mut M, value: u16) -> Result<(), MemoryError> {
        let hi = (value >> 8) as u8;
        let lo = value as u8;

        self.sp = self.sp.wrapping_sub(1);
        mem.write(self.sp, hi)?;
        self.sp = self.sp.wrapping_sub(1);
        mem.write(self.sp, lo)?;

        Ok(())
    }

    fn get_reg_u8(&mut self, reg: CpuRegister) -> Result<u8, CpuError> {
        match reg {
            CpuRegister::A => Ok(self.a),
            CpuRegister::B => Ok(self.b),
            CpuRegister::C => Ok(self.c),
            CpuRegister::D => Ok(self.d),
            CpuRegister::E => Ok(self.e),
            CpuRegister::H => Ok(self.h),
            CpuRegister::L => Ok(self.l),
            CpuRegister::F => Ok(self.f),
            _ => Err(CpuError::OperandSizeMismatch {
                operand: InstructionOperand::Register(reg),
                op: MemoryOperation::Read,
            }),
        }
    }

    fn set_reg_u8(&mut self, reg: CpuRegister, value: u8) -> Result<(), CpuError> {
        match reg {
            CpuRegister::A => self.a = value,
            CpuRegister::B => self.b = value,
            CpuRegister::C => self.c = value,
            CpuRegister::D => self.d = value,
            CpuRegister::E => self.e = value,
            CpuRegister::H => self.h = value,
            CpuRegister::L => self.l = value,
            CpuRegister::F => self.f = value,
            CpuRegister::AF => self.set_af(value as u16),
            CpuRegister::BC => self.set_bc(value as u16),
            CpuRegister::DE => self.set_de(value as u16),
            CpuRegister::HL => self.set_hl(value as u16),
            CpuRegister::SP => self.sp = value as u16,
        }

        Ok(())
    }

    fn get_reg_u16(&mut self, reg: CpuRegister) -> Result<u16, CpuError> {
        match reg {
            CpuRegister::A => Ok(self.a as u16),
            CpuRegister::B => Ok(self.b as u16),
            CpuRegister::C => Ok(self.c as u16),
            CpuRegister::D => Ok(self.d as u16),
            CpuRegister::E => Ok(self.e as u16),
            CpuRegister::H => Ok(self.h as u16),
            CpuRegister::L => Ok(self.l as u16),
            CpuRegister::F => Ok(self.f as u16),
            CpuRegister::AF => Ok(self.af()),
            CpuRegister::BC => Ok(self.bc()),
            CpuRegister::DE => Ok(self.de()),
            CpuRegister::HL => Ok(self.hl()),
            CpuRegister::SP => Ok(self.sp),
        }
    }

    fn set_reg_u16(&mut self, reg: CpuRegister, value: u16) -> Result<(), CpuError> {
        match reg {
            CpuRegister::AF => self.set_af(value),
            CpuRegister::BC => self.set_bc(value),
            CpuRegister::DE => self.set_de(value),
            CpuRegister::HL => self.set_hl(value),
            CpuRegister::SP => self.sp = value,
            _ => {
                return Err(CpuError::OperandSizeMismatch {
                    operand: InstructionOperand::Register(reg),
                    op: MemoryOperation::Write,
                })
            }
        }

        Ok(())
    }

    fn get_u8<M: Memory>(
        &mut self,
        mem: &mut M,
        operand: InstructionOperand,
    ) -> Result<u8, CpuError> {
        match operand {
            InstructionOperand::Register(reg) => self.get_reg_u8(reg),
            InstructionOperand::Immediate8(val) => Ok(val),
            InstructionOperand::Immediate16(_) => Err(CpuError::ImmediateSizeMismatch),
            InstructionOperand::OffsetMemoryLocationRegister(offset, reg) => {
                Ok(mem.read(self.get_reg_u16(reg)?.wrapping_add(offset))?)
            }
            InstructionOperand::MemoryLocationRegister(reg) => {
                Ok(mem.read(self.get_reg_u16(reg)?)?)
            }
            InstructionOperand::MemoryLocationRegisterDecrement(reg) => {
                let value = mem.read(self.get_reg_u16(reg)?)?;
                let reg_value = self.get_reg_u16(reg)?.wrapping_sub(1);
                self.set_reg_u16(reg, reg_value)?;
                Ok(value)
            }
            InstructionOperand::MemoryLocationRegisterIncrement(reg) => {
                let value = mem.read(self.get_reg_u16(reg)?)?;
                let reg_value = self.get_reg_u16(reg)?.wrapping_add(1);
                self.set_reg_u16(reg, reg_value)?;
                Ok(value)
            }
            InstructionOperand::OffsetMemoryLocationImmediate8(offset, address) => {
                Ok(mem.read(offset + address as u16)?)
            }
            InstructionOperand::MemoryLocationImmediate16(address) => Ok(mem.read(address)?),
        }
    }

    fn set_u8<M: Memory>(
        &mut self,
        mem: &mut M,
        operand: InstructionOperand,
        value: u8,
    ) -> Result<(), CpuError> {
        match operand {
            InstructionOperand::Register(reg) => self.set_reg_u8(reg, value),
            InstructionOperand::OffsetMemoryLocationRegister(offset, reg) => {
                Ok(mem.write(self.get_reg_u16(reg)?.wrapping_add(offset), value)?)
            }
            InstructionOperand::MemoryLocationRegister(reg) => {
                Ok(mem.write(self.get_reg_u16(reg)?, value)?)
            }
            InstructionOperand::MemoryLocationRegisterDecrement(reg) => {
                mem.write(self.get_reg_u16(reg)?, value)?;
                let reg_value = self.get_reg_u16(reg)?.wrapping_sub(1);
                self.set_reg_u16(reg, reg_value)?;
                Ok(())
            }
            InstructionOperand::MemoryLocationRegisterIncrement(reg) => {
                mem.write(self.get_reg_u16(reg)?, value)?;
                let reg_value = self.get_reg_u16(reg)?.wrapping_add(1);
                self.set_reg_u16(reg, reg_value)?;
                Ok(())
            }
            InstructionOperand::OffsetMemoryLocationImmediate8(offset, address) => {
                Ok(mem.write(offset + address as u16, value)?)
            }
            InstructionOperand::MemoryLocationImmediate16(address) => {
                Ok(mem.write(address, value)?)
            }
            InstructionOperand::Immediate8(_) => Err(CpuError::ImmediateWrite),
            InstructionOperand::Immediate16(_) => Err(CpuError::ImmediateWrite),
        }
    }

    fn get_u16<M: Memory>(
        &mut self,
        mem: &mut M,
        operand: InstructionOperand,
    ) -> Result<u16, CpuError> {
        match operand {
            InstructionOperand::Register(reg) => self.get_reg_u16(reg),
            InstructionOperand::Immediate8(val) => Ok(val as u16),
            InstructionOperand::Immediate16(val) => Ok(val),
            InstructionOperand::OffsetMemoryLocationRegister(offset, reg) => {
                Ok(mem.read(self.get_reg_u16(reg)?.wrapping_add(offset))? as u16)
            }
            InstructionOperand::MemoryLocationRegister(reg) => {
                Ok(mem.read(self.get_reg_u16(reg)?)? as u16)
            }
            InstructionOperand::MemoryLocationRegisterDecrement(reg) => {
                let value = mem.read(self.get_reg_u16(reg)?)? as u16;
                let reg_value = self.get_reg_u16(reg)?.wrapping_sub(1);
                self.set_reg_u16(reg, reg_value)?;
                Ok(value)
            }
            InstructionOperand::MemoryLocationRegisterIncrement(reg) => {
                let value = mem.read(self.get_reg_u16(reg)?)? as u16;
                let reg_value = self.get_reg_u16(reg)?.wrapping_add(1);
                self.set_reg_u16(reg, reg_value)?;
                Ok(value)
            }
            InstructionOperand::OffsetMemoryLocationImmediate8(offset, address) => {
                Ok(mem.read(offset + address as u16)? as u16)
            }
            InstructionOperand::MemoryLocationImmediate16(address) => Ok(mem.read(address)? as u16),
        }
    }

    fn set_u16(&mut self, operand: InstructionOperand, value: u16) -> Result<(), CpuError> {
        match operand {
            InstructionOperand::Register(reg) => self.set_reg_u16(reg, value),
            InstructionOperand::Immediate8(_) => Err(CpuError::ImmediateWrite),
            InstructionOperand::Immediate16(_) => Err(CpuError::ImmediateWrite),
            _ => Err(CpuError::OperandSizeMismatch {
                operand,
                op: MemoryOperation::Write,
            }),
        }
    }
}

impl Cpu {
    pub fn exec_next_instruction<M: Memory>(&mut self, mem: &mut M) -> Result<usize, CpuError> {
        let instruction = self.fetch_instruction(mem)?;
        self.exec_instruction(mem, instruction)
    }

    pub fn exec_instruction<M: Memory>(
        &mut self,
        mem: &mut M,
        instruction: Instruction,
    ) -> Result<usize, CpuError> {
        let mut cycles = instruction.cycles();

        match instruction {
            Instruction::Noop => {}
            Instruction::Load(to, from) => {
                if to.is_16bit() {
                    let val = self.get_u16(mem, from)?;
                    self.set_u16(to, val)?;
                } else {
                    let val = self.get_u8(mem, from)?;
                    self.set_u8(mem, to, val)?;
                }
            }
            Instruction::And(from) => {
                self.a &= self.get_u8(mem, from)?;

                self.set_flag(CpuFlag::Zero, self.a == 0);
                self.set_flag(CpuFlag::Subtraction, false);
                self.set_flag(CpuFlag::HalfCarry, true);
                self.set_flag(CpuFlag::Carry, false);
            }
            Instruction::Or(from) => {
                self.a |= self.get_u8(mem, from)?;

                self.set_flag(CpuFlag::Zero, self.a == 0);
                self.set_flag(CpuFlag::Subtraction, false);
                self.set_flag(CpuFlag::HalfCarry, false);
                self.set_flag(CpuFlag::Carry, false);
            }
            Instruction::Xor(from) => {
                self.a ^= self.get_u8(mem, from)?;

                self.set_flag(CpuFlag::Zero, self.a == 0);
                self.set_flag(CpuFlag::Subtraction, false);
                self.set_flag(CpuFlag::HalfCarry, false);
                self.set_flag(CpuFlag::Carry, false);
            }
            Instruction::Bit(bit, from) => {
                let set = self.get_u8(mem, from)? & (1 << bit) != 0;

                self.set_flag(CpuFlag::Zero, !set);
                self.set_flag(CpuFlag::Subtraction, false);
                self.set_flag(CpuFlag::HalfCarry, true);
            }
            Instruction::Jump(address) => {
                self.pc = address;
            }
            Instruction::JumpRelative(offset) => {
                self.pc = self.pc.wrapping_add(offset as u16);
            }
            Instruction::JumpRelativeIf(flag, expected, offset) => {
                if self.get_flag(flag) == expected {
                    cycles += 1;
                    self.pc = self.pc.wrapping_add(offset as u16);
                }
            }
            Instruction::Increment(to) => {
                if to.is_16bit() {
                    let val = self.get_u16(mem, to)?.wrapping_add(1);
                    self.set_u16(to, val)?;
                } else {
                    let ov = self.get_u8(mem, to)?;
                    let val = ov.wrapping_add(1);
                    self.set_u8(mem, to, val)?;

                    self.set_flag(CpuFlag::Zero, val == 0);
                    self.set_flag(CpuFlag::Subtraction, false);
                    self.set_flag(CpuFlag::HalfCarry, ov & 0x10 != val & 0x10);
                }
            }
            Instruction::Decrement(to) => {
                if to.is_16bit() {
                    let val = self.get_u16(mem, to)?.wrapping_sub(1);
                    self.set_u16(to, val)?;
                } else {
                    let ov = self.get_u8(mem, to)?;
                    let val = ov.wrapping_sub(1);
                    self.set_u8(mem, to, val)?;

                    self.set_flag(CpuFlag::Zero, val == 0);
                    self.set_flag(CpuFlag::Subtraction, true);
                    self.set_flag(CpuFlag::HalfCarry, ov & 0xf == 0);
                }
            }
            Instruction::Call(address) => {
                self.push_u16(mem, self.pc)?;
                self.pc = address;
            }
            Instruction::Push(reg) => {
                let value = self.get_reg_u16(reg)?;
                self.push_u16(mem, value)?
            }
            Instruction::Pop(reg) => {
                let value = self.pop_u16(mem)?;
                self.set_reg_u16(reg, value)?;
            }
            Instruction::ExtendedRotateLeft(to) => {
                let carry = self.get_flag(CpuFlag::Carry) as u8;
                let previous = self.get_u8(mem, to)?;

                self.set_flag(CpuFlag::Carry, previous & 0x80 != 0);

                let value = previous << 1 | carry;
                self.set_u8(mem, to, value)?;

                self.set_flag(CpuFlag::Zero, value == 0);
                self.set_flag(CpuFlag::Subtraction, false);
                self.set_flag(CpuFlag::HalfCarry, false);
            }
            Instruction::RotateLeftA => {
                let carry = self.get_flag(CpuFlag::Carry) as u8;
                self.set_flag(CpuFlag::Carry, self.a & 0x80 != 0);
                self.a = self.a << 1 | carry;
            }
            Instruction::Return => self.pc = self.pop_u16(mem)?,
            Instruction::Compare(to) => {
                let value = self.get_u8(mem, to)?;
                self.subtract_a(value, false);
            }
            Instruction::Subtract(from) => {
                let value = self.get_u8(mem, from)?;
                self.a = self.subtract_a(value, false);
            }
            Instruction::Add(to, from) => {
                let carry = 0; //self.get_flag(CpuFlag::Carry) as u8;

                if to.is_16bit() {
                    let value = self.get_reg_u16(to)?;
                    let result = value
                        .wrapping_add(self.get_u16(mem, from)?)
                        .wrapping_add(carry as u16);

                    self.set_reg_u16(to, result)?;

                    self.set_flag(CpuFlag::Subtraction, false);
                    self.set_flag(CpuFlag::HalfCarry, result & 0x10 != 0);
                    self.set_flag(
                        CpuFlag::Carry,
                        (result < value) || (carry == 1 && value == result),
                    );

                    if let CpuRegister::SP = to {
                        self.set_flag(CpuFlag::Zero, false);
                    }
                } else {
                    let value = self.get_reg_u8(to)?;
                    let result = value
                        .wrapping_add(self.get_u8(mem, from)?)
                        .wrapping_add(carry);

                    self.set_reg_u8(to, result)?;

                    self.set_flag(CpuFlag::Zero, result == 0);
                    self.set_flag(CpuFlag::Subtraction, false);
                    self.set_flag(CpuFlag::HalfCarry, result & 0x10 != 0);
                    self.set_flag(
                        CpuFlag::Carry,
                        (result < value) || (carry == 1 && value == result),
                    );
                }
            }
            Instruction::DisableInterrupts => {
                // TODO: Implement interrupts
            }
            Instruction::EnableInterrupts => {
                // TODO: Implement interrupts
            }
            Instruction::Complement => {
                self.a = !self.a;

                self.set_flag(CpuFlag::Subtraction, true);
                self.set_flag(CpuFlag::HalfCarry, true);
            }
            Instruction::Swap(to) => {
                let value = self.get_u8(mem, to)?;
                let result = value >> 4 | (value & 0xf) << 4;

                self.set_u8(mem, to, result)?;

                self.set_flag(CpuFlag::Zero, result == 0);
                self.set_flag(CpuFlag::Subtraction, false);
                self.set_flag(CpuFlag::HalfCarry, false);
                self.set_flag(CpuFlag::Carry, false);
            }
            _ => panic!("unimplemented instruction {:x?}", instruction),
        }

        Ok(cycles)
    }

    fn subtract_a(&mut self, value: u8, carry: bool) -> u8 {
        let carry = carry as u8;
        let previous = self.a;

        let result = self.a.wrapping_sub(value).wrapping_sub(carry);

        self.set_flag(CpuFlag::Zero, result == 0);
        self.set_flag(CpuFlag::Subtraction, true);
        self.set_flag(
            CpuFlag::HalfCarry,
            (result & 0xf).wrapping_sub(value & 0xf).wrapping_sub(carry) & 0x10 != 0,
        );
        self.set_flag(
            CpuFlag::Carry,
            (result > previous) || (carry == 1 && result == previous),
        );

        result
    }
}

impl Cpu {
    pub fn fetch_instruction<M: Memory>(
        &mut self,
        mem: &mut M,
    ) -> Result<Instruction, InstructionError> {
        let opcode = self.fetch_u8(mem)?;

        macro_rules! instr_operand {
            (( R $reg:ident )) => {
                CpuRegister::$reg
            };
            (( :R $reg:ident )) => {
                InstructionOperand::Register(CpuRegister::$reg)
            };
            (( @R $reg:ident )) => {
                InstructionOperand::MemoryLocationRegister(CpuRegister::$reg)
            };
            (( @R $reg:ident $offset:expr )) => {
                InstructionOperand::OffsetMemoryLocationRegister($offset, CpuRegister::$reg)
            };
            (( @R+ $reg:ident )) => {
                InstructionOperand::MemoryLocationRegisterIncrement(CpuRegister::$reg)
            };
            (( @R- $reg:ident )) => {
                InstructionOperand::MemoryLocationRegisterDecrement(CpuRegister::$reg)
            };
            (( @IMM16 )) => {
                InstructionOperand::MemoryLocationImmediate16(self.fetch_u16(mem)?)
            };
            (( @IMM8 $offset:expr )) => {
                InstructionOperand::OffsetMemoryLocationImmediate8($offset, self.fetch_u8(mem)?)
            };
            (( F $flag:ident )) => {
                CpuFlag::$flag
            };
            ( REL8 ) => {
                self.fetch_u8(mem)? as i8
            };
            ( ABS16 ) => {
                self.fetch_u16(mem)?
            };
            ( IMM8 ) => {
                InstructionOperand::Immediate8(self.fetch_u8(mem)?)
            };
            ( IMM16 ) => {
                InstructionOperand::Immediate16(self.fetch_u16(mem)?)
            };
            (( = $e:expr )) => {
                $e
            };
        }

        macro_rules! instr {
            ( $op:ident ) => {
                Ok(Instruction::$op)
            };
            ( $op:ident $($b:tt)* ) => {
                Ok(Instruction::$op($(instr_operand!($b)),*))
            };
        }

        match opcode {
            0x00 => instr!(Noop),
            0x01 => instr!(Load (:R BC) IMM16),
            0x02 => instr!(Load (@R BC) (:R A)),
            0x03 => instr!(Increment (:R BC)),
            0x04 => instr!(Increment (:R B)),
            0x05 => instr!(Decrement (:R B)),
            0x06 => instr!(Load (:R B) IMM8),
            // 0x08 => instr!(Load (@IMM16) (:R SP)),
            0x09 => instr!(Add (R HL) (:R BC)),
            0x0a => instr!(Add (R A) (@R BC)),
            0x0b => instr!(Decrement (:R BC)),
            0x0c => instr!(Increment (:R C)),
            0x0d => instr!(Decrement (:R C)),
            0x0e => instr!(Load (:R C) IMM8),
            0x10 => instr!(Stop),
            0x11 => instr!(Load (:R DE) IMM16),
            0x12 => instr!(Load (@R DE) (:R A)),
            0x13 => instr!(Increment (:R DE)),
            0x14 => instr!(Increment (:R D)),
            0x15 => instr!(Decrement (:R D)),
            0x16 => instr!(Load (:R D) IMM8),
            0x17 => instr!(RotateLeftA),
            0x18 => instr!(JumpRelative REL8),
            0x19 => instr!(Add (R HL) (:R DE)),
            0x1a => instr!(Load (:R A) (@R DE)),
            0x1b => instr!(Decrement (:R DE)),
            0x1c => instr!(Increment (:R E)),
            0x1d => instr!(Decrement (:R E)),
            0x1e => instr!(Load (:R E) IMM8),
            0x20 => instr!(JumpRelativeIf (F Zero) (= false) REL8),
            0x21 => instr!(Load (:R HL) IMM16),
            0x22 => instr!(Load (@R+ HL) (:R A)),
            0x23 => instr!(Increment (:R HL)),
            0x24 => instr!(Increment (:R H)),
            0x25 => instr!(Decrement (:R H)),
            0x26 => instr!(Load (:R H) IMM8),
            0x28 => instr!(JumpRelativeIf (F Zero) (= true) REL8),
            0x29 => instr!(Add (R HL) (:R HL)),
            0x2a => instr!(Load (:R A) (@R+ HL)),
            0x2b => instr!(Decrement (:R HL)),
            0x2c => instr!(Increment (:R L)),
            0x2d => instr!(Decrement (:R L)),
            0x2e => instr!(Load (:R L) IMM8),
            0x2f => instr!(Complement),
            0x31 => instr!(Load (:R SP) IMM16),
            0x32 => instr!(Load (@R- HL) (:R A)),
            0x33 => instr!(Increment (:R SP)),
            0x34 => instr!(Increment (@R HL)),
            0x35 => instr!(Decrement (@R HL)),
            0x36 => instr!(Load (@R HL) IMM8),
            0x38 => instr!(JumpRelativeIf (F Carry) (= true) REL8),
            0x39 => instr!(Add (R HL) (:R SP)),
            0x3a => instr!(Load (:R A) (@R- HL)),
            0x3b => instr!(Decrement (:R SP)),
            0x3c => instr!(Increment (:R A)),
            0x3d => instr!(Decrement (:R A)),
            0x3e => instr!(Load (:R A) IMM8),
            0x40 => instr!(Load (:R B) (:R B)),
            0x41 => instr!(Load (:R B) (:R C)),
            0x42 => instr!(Load (:R B) (:R D)),
            0x43 => instr!(Load (:R B) (:R E)),
            0x44 => instr!(Load (:R B) (:R H)),
            0x45 => instr!(Load (:R B) (:R L)),
            0x46 => instr!(Load (:R B) (@R HL)),
            0x47 => instr!(Load (:R B) (:R A)),
            0x48 => instr!(Load (:R C) (:R B)),
            0x49 => instr!(Load (:R C) (:R C)),
            0x4a => instr!(Load (:R C) (:R D)),
            0x4b => instr!(Load (:R C) (:R E)),
            0x4c => instr!(Load (:R C) (:R H)),
            0x4d => instr!(Load (:R C) (:R L)),
            0x4e => instr!(Load (:R C) (@R HL)),
            0x4f => instr!(Load (:R C) (:R A)),
            0x50 => instr!(Load (:R D) (:R B)),
            0x51 => instr!(Load (:R D) (:R C)),
            0x52 => instr!(Load (:R D) (:R D)),
            0x53 => instr!(Load (:R D) (:R E)),
            0x54 => instr!(Load (:R D) (:R H)),
            0x55 => instr!(Load (:R D) (:R L)),
            0x56 => instr!(Load (:R D) (@R HL)),
            0x57 => instr!(Load (:R D) (:R A)),
            0x58 => instr!(Load (:R E) (:R B)),
            0x59 => instr!(Load (:R E) (:R C)),
            0x5a => instr!(Load (:R E) (:R D)),
            0x5b => instr!(Load (:R E) (:R E)),
            0x5c => instr!(Load (:R E) (:R H)),
            0x5d => instr!(Load (:R E) (:R L)),
            0x5e => instr!(Load (:R E) (@R HL)),
            0x5f => instr!(Load (:R E) (:R A)),
            0x60 => instr!(Load (:R H) (:R B)),
            0x61 => instr!(Load (:R H) (:R C)),
            0x62 => instr!(Load (:R H) (:R D)),
            0x63 => instr!(Load (:R H) (:R E)),
            0x64 => instr!(Load (:R H) (:R H)),
            0x65 => instr!(Load (:R H) (:R L)),
            0x66 => instr!(Load (:R H) (@R HL)),
            0x67 => instr!(Load (:R H) (:R A)),
            0x68 => instr!(Load (:R L) (:R B)),
            0x69 => instr!(Load (:R L) (:R C)),
            0x6a => instr!(Load (:R L) (:R D)),
            0x6b => instr!(Load (:R L) (:R E)),
            0x6c => instr!(Load (:R L) (:R H)),
            0x6d => instr!(Load (:R L) (:R L)),
            0x6e => instr!(Load (:R L) (@R HL)),
            0x6f => instr!(Load (:R L) (:R A)),
            0x70 => instr!(Load (@R HL) (:R B)),
            0x71 => instr!(Load (@R HL) (:R C)),
            0x72 => instr!(Load (@R HL) (:R D)),
            0x73 => instr!(Load (@R HL) (:R E)),
            0x74 => instr!(Load (@R HL) (:R H)),
            0x75 => instr!(Load (@R HL) (:R L)),
            0x77 => instr!(Load (@R HL) (:R A)),
            0x78 => instr!(Load (:R A) (:R B)),
            0x79 => instr!(Load (:R A) (:R C)),
            0x7a => instr!(Load (:R A) (:R D)),
            0x7b => instr!(Load (:R A) (:R E)),
            0x7c => instr!(Load (:R A) (:R H)),
            0x7d => instr!(Load (:R A) (:R L)),
            0x7e => instr!(Load (:R A) (@R HL)),
            0x7f => instr!(Load (:R A) (:R A)),
            0x80 => instr!(Add (R A) (:R B)),
            0x81 => instr!(Add (R A) (:R C)),
            0x82 => instr!(Add (R A) (:R D)),
            0x83 => instr!(Add (R A) (:R E)),
            0x84 => instr!(Add (R A) (:R H)),
            0x85 => instr!(Add (R A) (:R L)),
            0x86 => instr!(Add (R A) (@R HL)),
            0x87 => instr!(Add (R A) (:R A)),
            0x90 => instr!(Subtract (:R B)),
            0x91 => instr!(Subtract (:R C)),
            0x92 => instr!(Subtract (:R D)),
            0x93 => instr!(Subtract (:R E)),
            0x94 => instr!(Subtract (:R H)),
            0x95 => instr!(Subtract (:R L)),
            0x96 => instr!(Subtract (@R HL)),
            0x97 => instr!(Subtract (:R A)),
            0xa0 => instr!(And (:R B)),
            0xa1 => instr!(And (:R C)),
            0xa2 => instr!(And (:R D)),
            0xa3 => instr!(And (:R E)),
            0xa4 => instr!(And (:R H)),
            0xa5 => instr!(And (:R L)),
            0xa6 => instr!(And (@R HL)),
            0xa7 => instr!(And (:R A)),
            0xa8 => instr!(Xor (:R B)),
            0xa9 => instr!(Xor (:R C)),
            0xaa => instr!(Xor (:R D)),
            0xab => instr!(Xor (:R E)),
            0xac => instr!(Xor (:R H)),
            0xad => instr!(Xor (:R L)),
            0xae => instr!(Xor (@R HL)),
            0xaf => instr!(Xor (:R A)),
            0xb0 => instr!(Or (:R B)),
            0xb1 => instr!(Or (:R C)),
            0xb2 => instr!(Or (:R D)),
            0xb3 => instr!(Or (:R E)),
            0xb4 => instr!(Or (:R H)),
            0xb5 => instr!(Or (:R L)),
            0xb6 => instr!(Or (@R HL)),
            0xb7 => instr!(Or (:R A)),
            0xb8 => instr!(Compare (:R B)),
            0xb9 => instr!(Compare (:R C)),
            0xba => instr!(Compare (:R D)),
            0xbb => instr!(Compare (:R E)),
            0xbc => instr!(Compare (:R H)),
            0xbd => instr!(Compare (:R L)),
            0xbe => instr!(Compare (@R HL)),
            0xbf => instr!(Compare (:R A)),
            0xc1 => instr!(Pop (R BC)),
            0xc3 => instr!(Jump ABS16),
            0xc5 => instr!(Push (R BC)),
            0xc6 => instr!(Add (R A) IMM8),
            0xc9 => instr!(Return),
            0xcb => {
                let opcode = self.fetch_u8(mem)?;

                match opcode {
                    0x11 => instr!(ExtendedRotateLeft (:R C)),
                    0x37 => instr!(Swap (:R A)),
                    0x7c => instr!(Bit (= 7) (:R H)),
                    _ => Err(InstructionError::InvalidOpcode {
                        opcode: opcode as u16 + 0xcb00,
                    }),
                }
            }
            0xcd => instr!(Call ABS16),
            0xd1 => instr!(Pop (R DE)),
            0xd5 => instr!(Push (R DE)),
            0xd6 => instr!(Subtract IMM8),
            0xe0 => instr!(Load (@IMM8 0xff00) (:R A)),
            0xe1 => instr!(Pop (R HL)),
            0xe2 => instr!(Load (@R C 0xff00) (:R A)),
            0xe5 => instr!(Push (R HL)),
            0xe6 => instr!(And IMM8),
            // 0xe8 => instr!(Add (R SP) IMM8),
            0xea => instr!(Load (@IMM16) (:R A)),
            0xee => instr!(Xor IMM8),
            0xf0 => instr!(Load (:R A) (@IMM8 0xff00)),
            0xf1 => instr!(Pop (R AF)),
            0xf2 => instr!(Load (:R A) (@R C 0xff00)),
            0xf3 => instr!(DisableInterrupts),
            0xf5 => instr!(Push (R AF)),
            0xf6 => instr!(Or IMM8),
            // 0xf8 => instr!(Load (:R HL) (:R SP IMM8))
            // 0xf9 => instr!(Load (:R SP) (:R HL)),
            0xfa => instr!(Load (:R A) (@IMM16)),
            0xfb => instr!(EnableInterrupts),
            0xfe => instr!(Compare IMM8),
            _ => Err(InstructionError::InvalidOpcode {
                opcode: opcode as u16,
            }),
        }
    }

    fn fetch_u8<M: Memory>(&mut self, mem: &mut M) -> Result<u8, MemoryError> {
        let ret = mem.read(self.pc)?;
        self.pc = self.pc.wrapping_add(1);
        Ok(ret)
    }

    fn fetch_u16<M: Memory>(&mut self, mem: &mut M) -> Result<u16, MemoryError> {
        let ret = (mem.read(self.pc + 1)? as u16) << 8 | (mem.read(self.pc)? as u16);
        self.pc = self.pc.wrapping_add(2);
        Ok(ret)
    }

    pub fn disassemble<M: Memory>(&mut self, mem: &mut M, max: u16) -> BTreeMap<u16, String> {
        let old_pc = self.pc;
        let mut res = BTreeMap::new();

        self.pc = 0;
        let mut pc = 0;
        while !res.contains_key(&pc) && pc < max {
            let instruction = self.fetch_instruction(mem);
            if let Ok(instruction) = instruction {
                res.insert(pc, format!("{}", instruction));
            } else {
                res.insert(pc, "<unknown>".to_string());
            }
            pc = self.pc;
        }

        self.pc = old_pc;

        res
    }
}
