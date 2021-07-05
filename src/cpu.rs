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

        match opcode {
            0x00 => Ok(Instruction::Noop),
            0x01 => Ok(Instruction::Load(
                InstructionOperand::Register(CpuRegister::BC),
                InstructionOperand::Immediate16(self.fetch_u16(mem)?),
            )),
            0x02 => Ok(Instruction::Load(
                InstructionOperand::MemoryLocationRegister(CpuRegister::BC),
                InstructionOperand::Register(CpuRegister::A),
            )),
            0x04 => Ok(Instruction::Increment(InstructionOperand::Register(
                CpuRegister::B,
            ))),
            0x05 => Ok(Instruction::Decrement(InstructionOperand::Register(
                CpuRegister::B,
            ))),
            0x06 => Ok(Instruction::Load(
                InstructionOperand::Register(CpuRegister::B),
                InstructionOperand::Immediate8(self.fetch_u8(mem)?),
            )),
            0x0b => Ok(Instruction::Decrement(InstructionOperand::Register(
                CpuRegister::BC,
            ))),
            0x0c => Ok(Instruction::Increment(InstructionOperand::Register(
                CpuRegister::C,
            ))),
            0x0d => Ok(Instruction::Decrement(InstructionOperand::Register(
                CpuRegister::C,
            ))),
            0x0e => Ok(Instruction::Load(
                InstructionOperand::Register(CpuRegister::C),
                InstructionOperand::Immediate8(self.fetch_u8(mem)?),
            )),
            0x11 => Ok(Instruction::Load(
                InstructionOperand::Register(CpuRegister::DE),
                InstructionOperand::Immediate16(self.fetch_u16(mem)?),
            )),
            0x13 => Ok(Instruction::Increment(InstructionOperand::Register(
                CpuRegister::DE,
            ))),
            0x15 => Ok(Instruction::Decrement(InstructionOperand::Register(
                CpuRegister::D,
            ))),
            0x16 => Ok(Instruction::Load(
                InstructionOperand::Register(CpuRegister::D),
                InstructionOperand::Immediate8(self.fetch_u8(mem)?),
            )),
            0x17 => Ok(Instruction::RotateLeftA),
            0x18 => Ok(Instruction::JumpRelative(self.fetch_u8(mem)? as i8)),
            0x1a => Ok(Instruction::Load(
                InstructionOperand::Register(CpuRegister::A),
                InstructionOperand::MemoryLocationRegister(CpuRegister::DE),
            )),
            0x1d => Ok(Instruction::Decrement(InstructionOperand::Register(
                CpuRegister::E,
            ))),
            0x1e => Ok(Instruction::Load(
                InstructionOperand::Register(CpuRegister::E),
                InstructionOperand::Immediate8(self.fetch_u8(mem)?),
            )),
            0x20 => Ok(Instruction::JumpRelativeIf(
                CpuFlag::Zero,
                false,
                self.fetch_u8(mem)? as i8,
            )),
            0x21 => Ok(Instruction::Load(
                InstructionOperand::Register(CpuRegister::HL),
                InstructionOperand::Immediate16(self.fetch_u16(mem)?),
            )),
            0x22 => Ok(Instruction::Load(
                InstructionOperand::MemoryLocationRegisterIncrement(CpuRegister::HL),
                InstructionOperand::Register(CpuRegister::A),
            )),
            0x23 => Ok(Instruction::Increment(InstructionOperand::Register(
                CpuRegister::HL,
            ))),
            0x24 => Ok(Instruction::Increment(InstructionOperand::Register(
                CpuRegister::H,
            ))),
            0x28 => Ok(Instruction::JumpRelativeIf(
                CpuFlag::Zero,
                true,
                self.fetch_u8(mem)? as i8,
            )),
            0x2a => Ok(Instruction::Load(
                InstructionOperand::Register(CpuRegister::A),
                InstructionOperand::MemoryLocationRegisterIncrement(CpuRegister::HL),
            )),
            0x2e => Ok(Instruction::Load(
                InstructionOperand::Register(CpuRegister::L),
                InstructionOperand::Immediate8(self.fetch_u8(mem)?),
            )),
            0x2f => Ok(Instruction::Complement),
            0x31 => Ok(Instruction::Load(
                InstructionOperand::Register(CpuRegister::SP),
                InstructionOperand::Immediate16(self.fetch_u16(mem)?),
            )),
            0x32 => Ok(Instruction::Load(
                InstructionOperand::MemoryLocationRegisterDecrement(CpuRegister::HL),
                InstructionOperand::Register(CpuRegister::A),
            )),
            0x36 => Ok(Instruction::Load(
                InstructionOperand::MemoryLocationRegister(CpuRegister::HL),
                InstructionOperand::Immediate8(self.fetch_u8(mem)?),
            )),
            0x3d => Ok(Instruction::Decrement(InstructionOperand::Register(
                CpuRegister::A,
            ))),
            0x3e => Ok(Instruction::Load(
                InstructionOperand::Register(CpuRegister::A),
                InstructionOperand::Immediate8(self.fetch_u8(mem)?),
            )),
            0x47 => Ok(Instruction::Load(
                InstructionOperand::Register(CpuRegister::B),
                InstructionOperand::Register(CpuRegister::A),
            )),
            0x4f => Ok(Instruction::Load(
                InstructionOperand::Register(CpuRegister::C),
                InstructionOperand::Register(CpuRegister::A),
            )),
            0x57 => Ok(Instruction::Load(
                InstructionOperand::Register(CpuRegister::D),
                InstructionOperand::Register(CpuRegister::A),
            )),
            0x67 => Ok(Instruction::Load(
                InstructionOperand::Register(CpuRegister::H),
                InstructionOperand::Register(CpuRegister::A),
            )),
            0x77 => Ok(Instruction::Load(
                InstructionOperand::MemoryLocationRegister(CpuRegister::HL),
                InstructionOperand::Register(CpuRegister::A),
            )),
            0x78 => Ok(Instruction::Load(
                InstructionOperand::Register(CpuRegister::A),
                InstructionOperand::Register(CpuRegister::B),
            )),
            0x7b => Ok(Instruction::Load(
                InstructionOperand::Register(CpuRegister::A),
                InstructionOperand::Register(CpuRegister::E),
            )),
            0x7c => Ok(Instruction::Load(
                InstructionOperand::Register(CpuRegister::A),
                InstructionOperand::Register(CpuRegister::H),
            )),
            0x7d => Ok(Instruction::Load(
                InstructionOperand::Register(CpuRegister::A),
                InstructionOperand::Register(CpuRegister::L),
            )),
            0x86 => Ok(Instruction::Add(
                CpuRegister::A,
                InstructionOperand::MemoryLocationRegister(CpuRegister::HL),
            )),
            0x90 => Ok(Instruction::Subtract(InstructionOperand::Register(
                CpuRegister::B,
            ))),
            0xa0 => Ok(Instruction::And(InstructionOperand::Register(
                CpuRegister::B,
            ))),
            0xa1 => Ok(Instruction::And(InstructionOperand::Register(
                CpuRegister::C,
            ))),
            0xa2 => Ok(Instruction::And(InstructionOperand::Register(
                CpuRegister::D,
            ))),
            0xa3 => Ok(Instruction::And(InstructionOperand::Register(
                CpuRegister::E,
            ))),
            0xa4 => Ok(Instruction::And(InstructionOperand::Register(
                CpuRegister::H,
            ))),
            0xa5 => Ok(Instruction::And(InstructionOperand::Register(
                CpuRegister::L,
            ))),
            0xa6 => Ok(Instruction::And(
                InstructionOperand::MemoryLocationRegister(CpuRegister::HL),
            )),
            0xa7 => Ok(Instruction::And(InstructionOperand::Register(
                CpuRegister::A,
            ))),
            0xa8 => Ok(Instruction::Xor(InstructionOperand::Register(
                CpuRegister::B,
            ))),
            0xa9 => Ok(Instruction::Xor(InstructionOperand::Register(
                CpuRegister::C,
            ))),
            0xaa => Ok(Instruction::Xor(InstructionOperand::Register(
                CpuRegister::D,
            ))),
            0xab => Ok(Instruction::Xor(InstructionOperand::Register(
                CpuRegister::E,
            ))),
            0xac => Ok(Instruction::Xor(InstructionOperand::Register(
                CpuRegister::H,
            ))),
            0xad => Ok(Instruction::Xor(InstructionOperand::Register(
                CpuRegister::L,
            ))),
            0xae => Ok(Instruction::Xor(
                InstructionOperand::MemoryLocationRegister(CpuRegister::HL),
            )),
            0xaf => Ok(Instruction::Xor(InstructionOperand::Register(
                CpuRegister::A,
            ))),
            0xb0 => Ok(Instruction::Or(InstructionOperand::Register(
                CpuRegister::B,
            ))),
            0xb1 => Ok(Instruction::Or(InstructionOperand::Register(
                CpuRegister::C,
            ))),
            0xb2 => Ok(Instruction::Or(InstructionOperand::Register(
                CpuRegister::D,
            ))),
            0xb3 => Ok(Instruction::Or(InstructionOperand::Register(
                CpuRegister::E,
            ))),
            0xb4 => Ok(Instruction::Or(InstructionOperand::Register(
                CpuRegister::H,
            ))),
            0xb5 => Ok(Instruction::Or(InstructionOperand::Register(
                CpuRegister::L,
            ))),
            0xb6 => Ok(Instruction::Or(InstructionOperand::MemoryLocationRegister(
                CpuRegister::HL,
            ))),
            0xb7 => Ok(Instruction::Or(InstructionOperand::Register(
                CpuRegister::A,
            ))),
            0xbe => Ok(Instruction::Compare(
                InstructionOperand::MemoryLocationRegister(CpuRegister::HL),
            )),
            0xc1 => Ok(Instruction::Pop(CpuRegister::BC)),
            0xc3 => Ok(Instruction::Jump(self.fetch_u16(mem)?)),
            0xc5 => Ok(Instruction::Push(CpuRegister::BC)),
            0xc9 => Ok(Instruction::Return),
            0xcb => self.fetch_extended_instruction(mem),
            0xcd => Ok(Instruction::Call(self.fetch_u16(mem)?)),
            0xe0 => Ok(Instruction::Load(
                InstructionOperand::OffsetMemoryLocationImmediate8(0xff00, self.fetch_u8(mem)?),
                InstructionOperand::Register(CpuRegister::A),
            )),
            0xe2 => Ok(Instruction::Load(
                InstructionOperand::OffsetMemoryLocationRegister(0xff00, CpuRegister::C),
                InstructionOperand::Register(CpuRegister::A),
            )),
            0xe6 => Ok(Instruction::And(InstructionOperand::Immediate8(
                self.fetch_u8(mem)?,
            ))),
            0xea => Ok(Instruction::Load(
                InstructionOperand::MemoryLocationImmediate16(self.fetch_u16(mem)?),
                InstructionOperand::Register(CpuRegister::A),
            )),
            0xf0 => Ok(Instruction::Load(
                InstructionOperand::Register(CpuRegister::A),
                InstructionOperand::OffsetMemoryLocationImmediate8(0xff00, self.fetch_u8(mem)?),
            )),
            0xf3 => Ok(Instruction::DisableInterrupts),
            0xfb => Ok(Instruction::EnableInterrupts),
            0xfe => Ok(Instruction::Compare(InstructionOperand::Immediate8(
                self.fetch_u8(mem)?,
            ))),
            _ => Err(InstructionError::InvalidOpcode {
                opcode: opcode as u16,
            }),
        }
    }

    fn fetch_extended_instruction<M: Memory>(
        &mut self,
        mem: &mut M,
    ) -> Result<Instruction, InstructionError> {
        let opcode = self.fetch_u8(mem)?;

        match opcode {
            0x11 => Ok(Instruction::ExtendedRotateLeft(
                InstructionOperand::Register(CpuRegister::C),
            )),
            0x37 => Ok(Instruction::Swap(InstructionOperand::Register(
                CpuRegister::A,
            ))),
            0x7c => Ok(Instruction::Bit(
                7,
                InstructionOperand::Register(CpuRegister::H),
            )),
            _ => Err(InstructionError::InvalidOpcode {
                opcode: opcode as u16 + 0xcb00,
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
