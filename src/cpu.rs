use anyhow::Context;
use bitflags::bitflags;
use std::{collections::BTreeMap, fmt, u8};
use thiserror::Error;

use crate::{
    instruction::{CpuRegister, Instruction, InstructionOperand, SPOps},
    memory::{Memory, MemoryError, MemoryOperation},
};

bitflags! {
    pub struct Interrupts: u8 {
        const VBLANK = 1 << 0;
        const LCD_STAT = 1 << 1;
        const TIMER = 1 << 2;
        const SERIAL = 1 << 3;
        const JOYPAD = 1 << 4;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum InterruptState {
    Disabled,
    ShouldEnable,
    Enabled,
}

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
    pub interrupt_state: InterruptState,
    pub halted: bool,
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
            interrupt_state: InterruptState::Disabled,
            halted: false,
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
        self.f = value as u8 & 0xf0;
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
            CpuRegister::F => self.f = value & 0xf0,
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
            InstructionOperand::DoubleMemoryLocationImmediate16(_) => {
                Err(CpuError::OperandSizeMismatch {
                    op: MemoryOperation::Read,
                    operand,
                })
            }
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
            InstructionOperand::DoubleMemoryLocationImmediate16(_) => {
                Err(CpuError::OperandSizeMismatch {
                    op: MemoryOperation::Write,
                    operand,
                })
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
            InstructionOperand::DoubleMemoryLocationImmediate16(address) => {
                Ok(mem.read(address)? as u16 + ((mem.read(address + 1)? as u16) << 8))
            }
        }
    }

    fn set_u16<M: Memory>(
        &mut self,
        mem: &mut M,
        operand: InstructionOperand,
        value: u16,
    ) -> Result<(), CpuError> {
        match operand {
            InstructionOperand::Register(reg) => self.set_reg_u16(reg, value),
            InstructionOperand::DoubleMemoryLocationImmediate16(address) => {
                mem.write(address, value as u8)?;
                mem.write(address + 1, (value >> 8) as u8)?;
                Ok(())
            }
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
        if let InterruptState::ShouldEnable = self.interrupt_state {
            self.interrupt_state = InterruptState::Enabled;
        }

        let mut cycles = instruction.cycles();

        match instruction {
            Instruction::Noop => {}
            Instruction::Stop => panic!("stop"),
            Instruction::Load(to, from) => {
                if to.is_16bit() {
                    let val = self.get_u16(mem, from)?;
                    self.set_u16(mem, to, val)?;
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
            Instruction::Jump(to) => {
                self.pc = self.get_u16(mem, to)?;
            }
            Instruction::JumpIf(flag, expected, address) => {
                if self.get_flag(flag) == expected {
                    cycles += 1;
                    self.pc = address;
                }
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
                    self.set_u16(mem, to, val)?;
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
                    self.set_u16(mem, to, val)?;
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
            Instruction::CallIf(flag, expected, address) => {
                if self.get_flag(flag) == expected {
                    cycles += 3;
                    self.push_u16(mem, self.pc)?;
                    self.pc = address;
                }
            }
            Instruction::Push(reg) => {
                let value = self.get_reg_u16(reg)?;
                self.push_u16(mem, value)?
            }
            Instruction::Pop(reg) => {
                let value = self.pop_u16(mem)?;
                self.set_reg_u16(reg, value)?;
            }
            Instruction::RotateLeft(to, use_carry) => {
                let previous = self.get_u8(mem, to)?;
                let bit = if use_carry {
                    self.get_flag(CpuFlag::Carry) as u8
                } else {
                    (previous & 0x80) >> 7
                };

                self.set_flag(CpuFlag::Carry, previous & 0x80 != 0);

                let value = previous << 1 | bit;
                self.set_u8(mem, to, value)?;

                self.set_flag(CpuFlag::Zero, value == 0);
                self.set_flag(CpuFlag::Subtraction, false);
                self.set_flag(CpuFlag::HalfCarry, false);
            }
            Instruction::RotateRight(to, use_carry) => {
                let previous = self.get_u8(mem, to)?;
                let bit = if use_carry {
                    (self.get_flag(CpuFlag::Carry) as u8) << 7
                } else {
                    (previous & 1) << 7
                };

                self.set_flag(CpuFlag::Carry, previous & 1 != 0);

                let value = previous >> 1 | bit;
                self.set_u8(mem, to, value)?;

                self.set_flag(CpuFlag::Zero, value == 0);
                self.set_flag(CpuFlag::Subtraction, false);
                self.set_flag(CpuFlag::HalfCarry, false);
            }
            Instruction::RotateLeftA(use_carry) => {
                let bit = if use_carry {
                    self.get_flag(CpuFlag::Carry) as u8
                } else {
                    (self.a & 0x80) >> 7
                };

                self.set_flag(CpuFlag::Carry, self.a & 0x80 != 0);

                self.a = self.a << 1 | bit;

                self.set_flag(CpuFlag::Zero, false);
                self.set_flag(CpuFlag::Subtraction, false);
                self.set_flag(CpuFlag::HalfCarry, false);
            }
            Instruction::RotateRightA(use_carry) => {
                let bit = if use_carry {
                    (self.get_flag(CpuFlag::Carry) as u8) << 7
                } else {
                    (self.a & 1) << 7
                };

                self.set_flag(CpuFlag::Carry, self.a & 1 != 0);

                self.a = self.a >> 1 | bit;

                self.set_flag(CpuFlag::Zero, false);
                self.set_flag(CpuFlag::Subtraction, false);
                self.set_flag(CpuFlag::HalfCarry, false);
            }
            Instruction::ShiftRight(to, zero) => {
                let value = self.get_u8(mem, to)?;

                self.set_flag(CpuFlag::Carry, value & 1 != 0);
                let last_bit = if zero { 0 } else { value & (1 << 7) };
                let result = value >> 1 | last_bit;

                self.set_u8(mem, to, result)?;

                self.set_flag(CpuFlag::Zero, result == 0);
                self.set_flag(CpuFlag::Subtraction, false);
                self.set_flag(CpuFlag::HalfCarry, false);
            }
            Instruction::ShiftLeft(to) => {
                let value = self.get_u8(mem, to)?;

                self.set_flag(CpuFlag::Carry, value & 0x80 != 0);
                let result = value << 1;

                self.set_u8(mem, to, result)?;

                self.set_flag(CpuFlag::Zero, result == 0);
                self.set_flag(CpuFlag::Subtraction, false);
                self.set_flag(CpuFlag::HalfCarry, false);
            }
            Instruction::Return => self.pc = self.pop_u16(mem)?,
            Instruction::ReturnIf(flag, expected) => {
                if self.get_flag(flag) == expected {
                    cycles += 3;
                    self.pc = self.pop_u16(mem)?
                }
            }
            Instruction::Compare(to) => {
                let value = self.get_u8(mem, to)?;
                self.subtract_a(value, false);
            }
            Instruction::ReturnInterrupt => {
                self.interrupt_state = InterruptState::Enabled;
                self.pc = self.pop_u16(mem)?
            }
            Instruction::Subtract(from, use_carry) => {
                let value = self.get_u8(mem, from)?;
                self.a = self.subtract_a(
                    value,
                    if use_carry {
                        self.get_flag(CpuFlag::Carry)
                    } else {
                        false
                    },
                );
            }
            Instruction::Add8(to, from, use_carry) => {
                let carry = if use_carry {
                    self.get_flag(CpuFlag::Carry) as u8
                } else {
                    0
                };

                let value = self.get_reg_u8(to)?;
                let right = self.get_u8(mem, from)?;
                let result = value.wrapping_add(right).wrapping_add(carry);

                self.set_reg_u8(to, result)?;

                self.set_flag(CpuFlag::Zero, result == 0);
                self.set_flag(CpuFlag::Subtraction, false);
                self.set_flag(
                    CpuFlag::HalfCarry,
                    (value & 0xf).wrapping_add(right & 0xf).wrapping_add(carry) > 0xf,
                );
                self.set_flag(
                    CpuFlag::Carry,
                    result as u16 != value as u16 + right as u16 + carry as u16,
                );
            }
            Instruction::Add16(to, from) => {
                let value = self.get_reg_u16(to)?;
                let right = self.get_u16(mem, from)?;
                let result = value.wrapping_add(right);

                self.set_reg_u16(to, result)?;

                self.set_flag(CpuFlag::Subtraction, false);
                self.set_flag(
                    CpuFlag::HalfCarry,
                    (value & 0xfff).wrapping_add(right & 0xfff) > 0xfff,
                );
                self.set_flag(CpuFlag::Carry, result < value);
            }
            Instruction::DisableInterrupts => self.interrupt_state = InterruptState::Disabled,
            Instruction::EnableInterrupts => self.interrupt_state = InterruptState::ShouldEnable,
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
            Instruction::Rst(address) => {
                self.push_u16(mem, self.pc)?;
                self.pc = address as u16 * 8;
            }
            Instruction::DAA => {
                let mut correction = 0i8;

                if self.get_flag(CpuFlag::HalfCarry)
                    || (!self.get_flag(CpuFlag::Subtraction) && (self.a & 0xf > 9))
                {
                    correction += 6;
                }

                if self.get_flag(CpuFlag::Carry)
                    || (!self.get_flag(CpuFlag::Subtraction) && (self.a > 0x99))
                {
                    correction += 0x60;
                    self.set_flag(CpuFlag::Carry, true);
                }

                self.a = self.a.wrapping_add(if self.get_flag(CpuFlag::Subtraction) {
                    -correction as u8
                } else {
                    correction as u8
                });

                self.set_flag(CpuFlag::Zero, self.a == 0);
                self.set_flag(CpuFlag::HalfCarry, false);
            }
            Instruction::SetBit(bit, to, set) => {
                let mut value = self.get_u8(mem, to)?;

                if set {
                    value |= 1 << bit;
                } else {
                    value &= !(1 << bit);
                }

                self.set_u8(mem, to, value)?;
            }
            Instruction::SPOps(op) => match op {
                SPOps::AddOffset(offset) => {
                    self.sp = self.offset_sp(offset);
                }
                SPOps::LoadIntoHL(offset) => {
                    let value = self.offset_sp(offset);
                    self.set_hl(value);
                }
                SPOps::LoadFromHL => {
                    self.sp = self.hl();
                }
            },
            Instruction::SetCarryFlag(toggle) => {
                self.set_flag(
                    CpuFlag::Carry,
                    if toggle {
                        !self.get_flag(CpuFlag::Carry)
                    } else {
                        true
                    },
                );

                self.set_flag(CpuFlag::Subtraction, false);
                self.set_flag(CpuFlag::HalfCarry, false);
            }
            Instruction::Halt => {
                self.halted = true;
            }
        }

        Ok(cycles)
    }

    fn offset_sp(&mut self, offset: i8) -> u16 {
        let signed_result = (self.sp as i16).wrapping_add(offset as i16);

        self.set_flag(CpuFlag::Zero, false);
        self.set_flag(CpuFlag::Subtraction, false);

        self.set_flag(
            CpuFlag::HalfCarry,
            ((self.sp as i16) ^ signed_result ^ (offset as i16)) & 0x10 != 0,
        );
        self.set_flag(
            CpuFlag::Carry,
            ((self.sp as i16) ^ signed_result ^ (offset as i16)) & 0x100 != 0,
        );

        self.sp.wrapping_add(offset as i16 as u16)
    }

    fn subtract_a(&mut self, value: u8, carry: bool) -> u8 {
        let carry = carry as u8;

        let result = self.a.wrapping_sub(value).wrapping_sub(carry);

        self.set_flag(CpuFlag::Zero, result == 0);
        self.set_flag(CpuFlag::Subtraction, true);
        self.set_flag(
            CpuFlag::HalfCarry,
            (self.a & 0xf).wrapping_sub(value & 0xf).wrapping_sub(carry) & 0x10 != 0,
        );
        self.set_flag(
            CpuFlag::Carry,
            (self.a as u16) < (value as u16 + carry as u16),
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
            (( @@IMM16 )) => {
                InstructionOperand::DoubleMemoryLocationImmediate16(self.fetch_u16(mem)?)
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
            0x07 => instr!(RotateLeftA (= false)),
            0x08 => instr!(Load (@@IMM16) (:R SP)),
            0x09 => instr!(Add16 (R HL) (:R BC)),
            0x0a => instr!(Load (:R A) (@R BC)),
            0x0b => instr!(Decrement (:R BC)),
            0x0c => instr!(Increment (:R C)),
            0x0d => instr!(Decrement (:R C)),
            0x0e => instr!(Load (:R C) IMM8),
            0x0f => instr!(RotateRightA (= false)),
            0x10 => instr!(Stop),
            0x11 => instr!(Load (:R DE) IMM16),
            0x12 => instr!(Load (@R DE) (:R A)),
            0x13 => instr!(Increment (:R DE)),
            0x14 => instr!(Increment (:R D)),
            0x15 => instr!(Decrement (:R D)),
            0x16 => instr!(Load (:R D) IMM8),
            0x17 => instr!(RotateLeftA (= true)),
            0x18 => instr!(JumpRelative REL8),
            0x19 => instr!(Add16 (R HL) (:R DE)),
            0x1a => instr!(Load (:R A) (@R DE)),
            0x1b => instr!(Decrement (:R DE)),
            0x1c => instr!(Increment (:R E)),
            0x1d => instr!(Decrement (:R E)),
            0x1e => instr!(Load (:R E) IMM8),
            0x1f => instr!(RotateRightA (= true)),
            0x20 => instr!(JumpRelativeIf (F Zero) (= false) REL8),
            0x21 => instr!(Load (:R HL) IMM16),
            0x22 => instr!(Load (@R+ HL) (:R A)),
            0x23 => instr!(Increment (:R HL)),
            0x24 => instr!(Increment (:R H)),
            0x25 => instr!(Decrement (:R H)),
            0x26 => instr!(Load (:R H) IMM8),
            0x27 => instr!(DAA),
            0x28 => instr!(JumpRelativeIf (F Zero) (= true) REL8),
            0x29 => instr!(Add16 (R HL) (:R HL)),
            0x2a => instr!(Load (:R A) (@R+ HL)),
            0x2b => instr!(Decrement (:R HL)),
            0x2c => instr!(Increment (:R L)),
            0x2d => instr!(Decrement (:R L)),
            0x2e => instr!(Load (:R L) IMM8),
            0x2f => instr!(Complement),
            0x30 => instr!(JumpRelativeIf (F Carry) (= false) REL8),
            0x31 => instr!(Load (:R SP) IMM16),
            0x32 => instr!(Load (@R- HL) (:R A)),
            0x33 => instr!(Increment (:R SP)),
            0x34 => instr!(Increment (@R HL)),
            0x35 => instr!(Decrement (@R HL)),
            0x36 => instr!(Load (@R HL) IMM8),
            0x37 => instr!(SetCarryFlag (= false)),
            0x38 => instr!(JumpRelativeIf (F Carry) (= true) REL8),
            0x39 => instr!(Add16 (R HL) (:R SP)),
            0x3a => instr!(Load (:R A) (@R- HL)),
            0x3b => instr!(Decrement (:R SP)),
            0x3c => instr!(Increment (:R A)),
            0x3d => instr!(Decrement (:R A)),
            0x3e => instr!(Load (:R A) IMM8),
            0x3f => instr!(SetCarryFlag (= true)),
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
            0x76 => instr!(Halt),
            0x77 => instr!(Load (@R HL) (:R A)),
            0x78 => instr!(Load (:R A) (:R B)),
            0x79 => instr!(Load (:R A) (:R C)),
            0x7a => instr!(Load (:R A) (:R D)),
            0x7b => instr!(Load (:R A) (:R E)),
            0x7c => instr!(Load (:R A) (:R H)),
            0x7d => instr!(Load (:R A) (:R L)),
            0x7e => instr!(Load (:R A) (@R HL)),
            0x7f => instr!(Load (:R A) (:R A)),
            0x80 => instr!(Add8 (R A) (:R B) (= false)),
            0x81 => instr!(Add8 (R A) (:R C) (= false)),
            0x82 => instr!(Add8 (R A) (:R D) (= false)),
            0x83 => instr!(Add8 (R A) (:R E) (= false)),
            0x84 => instr!(Add8 (R A) (:R H) (= false)),
            0x85 => instr!(Add8 (R A) (:R L) (= false)),
            0x86 => instr!(Add8 (R A) (@R HL) (= false)),
            0x87 => instr!(Add8 (R A) (:R A) (= false)),
            0x88 => instr!(Add8 (R A) (:R B) (= true)),
            0x89 => instr!(Add8 (R A) (:R C) (= true)),
            0x8a => instr!(Add8 (R A) (:R D) (= true)),
            0x8b => instr!(Add8 (R A) (:R E) (= true)),
            0x8c => instr!(Add8 (R A) (:R H) (= true)),
            0x8d => instr!(Add8 (R A) (:R L) (= true)),
            0x8e => instr!(Add8 (R A) (@R HL) (= true)),
            0x8f => instr!(Add8 (R A) (:R A) (= true)),
            0x90 => instr!(Subtract (:R B) (= false)),
            0x91 => instr!(Subtract (:R C) (= false)),
            0x92 => instr!(Subtract (:R D) (= false)),
            0x93 => instr!(Subtract (:R E) (= false)),
            0x94 => instr!(Subtract (:R H) (= false)),
            0x95 => instr!(Subtract (:R L) (= false)),
            0x96 => instr!(Subtract (@R HL) (= false)),
            0x97 => instr!(Subtract (:R A) (= false)),
            0x98 => instr!(Subtract (:R B) (= true)),
            0x99 => instr!(Subtract (:R C) (= true)),
            0x9a => instr!(Subtract (:R D) (= true)),
            0x9b => instr!(Subtract (:R E) (= true)),
            0x9c => instr!(Subtract (:R H) (= true)),
            0x9d => instr!(Subtract (:R L) (= true)),
            0x9e => instr!(Subtract (@R HL) (= true)),
            0x9f => instr!(Subtract (:R A) (= true)),
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
            0xc0 => instr!(ReturnIf (F Zero) (= false)),
            0xc1 => instr!(Pop (R BC)),
            0xc2 => instr!(JumpIf (F Zero) (= false) ABS16),
            0xc3 => instr!(Jump IMM16),
            0xc4 => instr!(CallIf (F Zero) (= false) ABS16),
            0xc5 => instr!(Push (R BC)),
            0xc6 => instr!(Add8 (R A) IMM8 (= false)),
            0xc7 => instr!(Rst (= 0)),
            0xc8 => instr!(ReturnIf (F Zero) (= true)),
            0xc9 => instr!(Return),
            0xca => instr!(JumpIf (F Zero) (= true) ABS16),
            0xcb => {
                let opcode = self.fetch_u8(mem)?;

                match opcode {
                    0x00 => instr!(RotateLeft (:R B) (= false)),
                    0x01 => instr!(RotateLeft (:R C) (= false)),
                    0x02 => instr!(RotateLeft (:R D) (= false)),
                    0x03 => instr!(RotateLeft (:R E) (= false)),
                    0x04 => instr!(RotateLeft (:R H) (= false)),
                    0x05 => instr!(RotateLeft (:R L) (= false)),
                    0x06 => instr!(RotateLeft (@R HL) (= false)),
                    0x07 => instr!(RotateLeft (:R A) (= false)),
                    0x08 => instr!(RotateRight (:R B) (= false)),
                    0x09 => instr!(RotateRight (:R C) (= false)),
                    0x0a => instr!(RotateRight (:R D) (= false)),
                    0x0b => instr!(RotateRight (:R E) (= false)),
                    0x0c => instr!(RotateRight (:R H) (= false)),
                    0x0d => instr!(RotateRight (:R L) (= false)),
                    0x0e => instr!(RotateRight (@R HL) (= false)),
                    0x0f => instr!(RotateRight (:R A) (= false)),
                    0x10 => instr!(RotateLeft (:R B) (= true)),
                    0x11 => instr!(RotateLeft (:R C) (= true)),
                    0x12 => instr!(RotateLeft (:R D) (= true)),
                    0x13 => instr!(RotateLeft (:R E) (= true)),
                    0x14 => instr!(RotateLeft (:R H) (= true)),
                    0x15 => instr!(RotateLeft (:R L) (= true)),
                    0x16 => instr!(RotateLeft (@R HL) (= true)),
                    0x17 => instr!(RotateLeft (:R A) (= true)),
                    0x18 => instr!(RotateRight (:R B) (= true)),
                    0x19 => instr!(RotateRight (:R C) (= true)),
                    0x1a => instr!(RotateRight (:R D) (= true)),
                    0x1b => instr!(RotateRight (:R E) (= true)),
                    0x1c => instr!(RotateRight (:R H) (= true)),
                    0x1d => instr!(RotateRight (:R L) (= true)),
                    0x1e => instr!(RotateRight (@R HL) (= true)),
                    0x1f => instr!(RotateRight (:R A) (= true)),
                    0x20 => instr!(ShiftLeft (:R B)),
                    0x21 => instr!(ShiftLeft (:R C)),
                    0x22 => instr!(ShiftLeft (:R D)),
                    0x23 => instr!(ShiftLeft (:R E)),
                    0x24 => instr!(ShiftLeft (:R H)),
                    0x25 => instr!(ShiftLeft (:R L)),
                    0x26 => instr!(ShiftLeft (@R HL)),
                    0x27 => instr!(ShiftLeft (:R A)),
                    0x28 => instr!(ShiftRight (:R B) (= false)),
                    0x29 => instr!(ShiftRight (:R C) (= false)),
                    0x2a => instr!(ShiftRight (:R D) (= false)),
                    0x2b => instr!(ShiftRight (:R E) (= false)),
                    0x2c => instr!(ShiftRight (:R H) (= false)),
                    0x2d => instr!(ShiftRight (:R L) (= false)),
                    0x2e => instr!(ShiftRight (@R HL) (= false)),
                    0x2f => instr!(ShiftRight (:R A) (= false)),
                    0x30 => instr!(Swap (:R B)),
                    0x31 => instr!(Swap (:R C)),
                    0x32 => instr!(Swap (:R D)),
                    0x33 => instr!(Swap (:R E)),
                    0x34 => instr!(Swap (:R H)),
                    0x35 => instr!(Swap (:R L)),
                    0x36 => instr!(Swap (@R HL)),
                    0x37 => instr!(Swap (:R A)),
                    0x38 => instr!(ShiftRight (:R B) (= true)),
                    0x39 => instr!(ShiftRight (:R C) (= true)),
                    0x3a => instr!(ShiftRight (:R D) (= true)),
                    0x3b => instr!(ShiftRight (:R E) (= true)),
                    0x3c => instr!(ShiftRight (:R H) (= true)),
                    0x3d => instr!(ShiftRight (:R L) (= true)),
                    0x3e => instr!(ShiftRight (@R HL) (= true)),
                    0x3f => instr!(ShiftRight (:R A) (= true)),
                    0x40 => instr!(Bit (= 0) (:R B)),
                    0x41 => instr!(Bit (= 0) (:R C)),
                    0x42 => instr!(Bit (= 0) (:R D)),
                    0x43 => instr!(Bit (= 0) (:R E)),
                    0x44 => instr!(Bit (= 0) (:R H)),
                    0x45 => instr!(Bit (= 0) (:R L)),
                    0x46 => instr!(Bit (= 0) (@R HL)),
                    0x47 => instr!(Bit (= 0) (:R A)),
                    0x48 => instr!(Bit (= 1) (:R B)),
                    0x49 => instr!(Bit (= 1) (:R C)),
                    0x4a => instr!(Bit (= 1) (:R D)),
                    0x4b => instr!(Bit (= 1) (:R E)),
                    0x4c => instr!(Bit (= 1) (:R H)),
                    0x4d => instr!(Bit (= 1) (:R L)),
                    0x4e => instr!(Bit (= 1) (@R HL)),
                    0x4f => instr!(Bit (= 1) (:R A)),
                    0x50 => instr!(Bit (= 2) (:R B)),
                    0x51 => instr!(Bit (= 2) (:R C)),
                    0x52 => instr!(Bit (= 2) (:R D)),
                    0x53 => instr!(Bit (= 2) (:R E)),
                    0x54 => instr!(Bit (= 2) (:R H)),
                    0x55 => instr!(Bit (= 2) (:R L)),
                    0x56 => instr!(Bit (= 2) (@R HL)),
                    0x57 => instr!(Bit (= 2) (:R A)),
                    0x58 => instr!(Bit (= 3) (:R B)),
                    0x59 => instr!(Bit (= 3) (:R C)),
                    0x5a => instr!(Bit (= 3) (:R D)),
                    0x5b => instr!(Bit (= 3) (:R E)),
                    0x5c => instr!(Bit (= 3) (:R H)),
                    0x5d => instr!(Bit (= 3) (:R L)),
                    0x5e => instr!(Bit (= 3) (@R HL)),
                    0x5f => instr!(Bit (= 3) (:R A)),
                    0x60 => instr!(Bit (= 4) (:R B)),
                    0x61 => instr!(Bit (= 4) (:R C)),
                    0x62 => instr!(Bit (= 4) (:R D)),
                    0x63 => instr!(Bit (= 4) (:R E)),
                    0x64 => instr!(Bit (= 4) (:R H)),
                    0x65 => instr!(Bit (= 4) (:R L)),
                    0x66 => instr!(Bit (= 4) (@R HL)),
                    0x67 => instr!(Bit (= 4) (:R A)),
                    0x68 => instr!(Bit (= 5) (:R B)),
                    0x69 => instr!(Bit (= 5) (:R C)),
                    0x6a => instr!(Bit (= 5) (:R D)),
                    0x6b => instr!(Bit (= 5) (:R E)),
                    0x6c => instr!(Bit (= 5) (:R H)),
                    0x6d => instr!(Bit (= 5) (:R L)),
                    0x6e => instr!(Bit (= 5) (@R HL)),
                    0x6f => instr!(Bit (= 5) (:R A)),
                    0x70 => instr!(Bit (= 6) (:R B)),
                    0x71 => instr!(Bit (= 6) (:R C)),
                    0x72 => instr!(Bit (= 6) (:R D)),
                    0x73 => instr!(Bit (= 6) (:R E)),
                    0x74 => instr!(Bit (= 6) (:R H)),
                    0x75 => instr!(Bit (= 6) (:R L)),
                    0x76 => instr!(Bit (= 6) (@R HL)),
                    0x77 => instr!(Bit (= 6) (:R A)),
                    0x78 => instr!(Bit (= 7) (:R B)),
                    0x79 => instr!(Bit (= 7) (:R C)),
                    0x7a => instr!(Bit (= 7) (:R D)),
                    0x7b => instr!(Bit (= 7) (:R E)),
                    0x7c => instr!(Bit (= 7) (:R H)),
                    0x7d => instr!(Bit (= 7) (:R L)),
                    0x7e => instr!(Bit (= 7) (@R HL)),
                    0x7f => instr!(Bit (= 7) (:R A)),
                    0x80 => instr!(SetBit (= 0) (:R B) (= false)),
                    0x81 => instr!(SetBit (= 0) (:R C) (= false)),
                    0x82 => instr!(SetBit (= 0) (:R D) (= false)),
                    0x83 => instr!(SetBit (= 0) (:R E) (= false)),
                    0x84 => instr!(SetBit (= 0) (:R H) (= false)),
                    0x85 => instr!(SetBit (= 0) (:R L) (= false)),
                    0x86 => instr!(SetBit (= 0) (@R HL) (= false)),
                    0x87 => instr!(SetBit (= 0) (:R A) (= false)),
                    0x88 => instr!(SetBit (= 1) (:R B) (= false)),
                    0x89 => instr!(SetBit (= 1) (:R C) (= false)),
                    0x8a => instr!(SetBit (= 1) (:R D) (= false)),
                    0x8b => instr!(SetBit (= 1) (:R E) (= false)),
                    0x8c => instr!(SetBit (= 1) (:R H) (= false)),
                    0x8d => instr!(SetBit (= 1) (:R L) (= false)),
                    0x8e => instr!(SetBit (= 1) (@R HL) (= false)),
                    0x8f => instr!(SetBit (= 1) (:R A) (= false)),
                    0x90 => instr!(SetBit (= 2) (:R B) (= false)),
                    0x91 => instr!(SetBit (= 2) (:R C) (= false)),
                    0x92 => instr!(SetBit (= 2) (:R D) (= false)),
                    0x93 => instr!(SetBit (= 2) (:R E) (= false)),
                    0x94 => instr!(SetBit (= 2) (:R H) (= false)),
                    0x95 => instr!(SetBit (= 2) (:R L) (= false)),
                    0x96 => instr!(SetBit (= 2) (@R HL) (= false)),
                    0x97 => instr!(SetBit (= 2) (:R A) (= false)),
                    0x98 => instr!(SetBit (= 3) (:R B) (= false)),
                    0x99 => instr!(SetBit (= 3) (:R C) (= false)),
                    0x9a => instr!(SetBit (= 3) (:R D) (= false)),
                    0x9b => instr!(SetBit (= 3) (:R E) (= false)),
                    0x9c => instr!(SetBit (= 3) (:R H) (= false)),
                    0x9d => instr!(SetBit (= 3) (:R L) (= false)),
                    0x9e => instr!(SetBit (= 3) (@R HL) (= false)),
                    0x9f => instr!(SetBit (= 3) (:R A) (= false)),
                    0xa0 => instr!(SetBit (= 4) (:R B) (= false)),
                    0xa1 => instr!(SetBit (= 4) (:R C) (= false)),
                    0xa2 => instr!(SetBit (= 4) (:R D) (= false)),
                    0xa3 => instr!(SetBit (= 4) (:R E) (= false)),
                    0xa4 => instr!(SetBit (= 4) (:R H) (= false)),
                    0xa5 => instr!(SetBit (= 4) (:R L) (= false)),
                    0xa6 => instr!(SetBit (= 4) (@R HL) (= false)),
                    0xa7 => instr!(SetBit (= 4) (:R A) (= false)),
                    0xa8 => instr!(SetBit (= 5) (:R B) (= false)),
                    0xa9 => instr!(SetBit (= 5) (:R C) (= false)),
                    0xaa => instr!(SetBit (= 5) (:R D) (= false)),
                    0xab => instr!(SetBit (= 5) (:R E) (= false)),
                    0xac => instr!(SetBit (= 5) (:R H) (= false)),
                    0xad => instr!(SetBit (= 5) (:R L) (= false)),
                    0xae => instr!(SetBit (= 5) (@R HL) (= false)),
                    0xaf => instr!(SetBit (= 5) (:R A) (= false)),
                    0xb0 => instr!(SetBit (= 6) (:R B) (= false)),
                    0xb1 => instr!(SetBit (= 6) (:R C) (= false)),
                    0xb2 => instr!(SetBit (= 6) (:R D) (= false)),
                    0xb3 => instr!(SetBit (= 6) (:R E) (= false)),
                    0xb4 => instr!(SetBit (= 6) (:R H) (= false)),
                    0xb5 => instr!(SetBit (= 6) (:R L) (= false)),
                    0xb6 => instr!(SetBit (= 6) (@R HL) (= false)),
                    0xb7 => instr!(SetBit (= 6) (:R A) (= false)),
                    0xb8 => instr!(SetBit (= 7) (:R B) (= false)),
                    0xb9 => instr!(SetBit (= 7) (:R C) (= false)),
                    0xba => instr!(SetBit (= 7) (:R D) (= false)),
                    0xbb => instr!(SetBit (= 7) (:R E) (= false)),
                    0xbc => instr!(SetBit (= 7) (:R H) (= false)),
                    0xbd => instr!(SetBit (= 7) (:R L) (= false)),
                    0xbe => instr!(SetBit (= 7) (@R HL) (= false)),
                    0xbf => instr!(SetBit (= 7) (:R A) (= false)),
                    0xc0 => instr!(SetBit (= 0) (:R B) (= true)),
                    0xc1 => instr!(SetBit (= 0) (:R C) (= true)),
                    0xc2 => instr!(SetBit (= 0) (:R D) (= true)),
                    0xc3 => instr!(SetBit (= 0) (:R E) (= true)),
                    0xc4 => instr!(SetBit (= 0) (:R H) (= true)),
                    0xc5 => instr!(SetBit (= 0) (:R L) (= true)),
                    0xc6 => instr!(SetBit (= 0) (@R HL) (= true)),
                    0xc7 => instr!(SetBit (= 0) (:R A) (= true)),
                    0xc8 => instr!(SetBit (= 1) (:R B) (= true)),
                    0xc9 => instr!(SetBit (= 1) (:R C) (= true)),
                    0xca => instr!(SetBit (= 1) (:R D) (= true)),
                    0xcb => instr!(SetBit (= 1) (:R E) (= true)),
                    0xcc => instr!(SetBit (= 1) (:R H) (= true)),
                    0xcd => instr!(SetBit (= 1) (:R L) (= true)),
                    0xce => instr!(SetBit (= 1) (@R HL) (= true)),
                    0xcf => instr!(SetBit (= 1) (:R A) (= true)),
                    0xd0 => instr!(SetBit (= 2) (:R B) (= true)),
                    0xd1 => instr!(SetBit (= 2) (:R C) (= true)),
                    0xd2 => instr!(SetBit (= 2) (:R D) (= true)),
                    0xd3 => instr!(SetBit (= 2) (:R E) (= true)),
                    0xd4 => instr!(SetBit (= 2) (:R H) (= true)),
                    0xd5 => instr!(SetBit (= 2) (:R L) (= true)),
                    0xd6 => instr!(SetBit (= 2) (@R HL) (= true)),
                    0xd7 => instr!(SetBit (= 2) (:R A) (= true)),
                    0xd8 => instr!(SetBit (= 3) (:R B) (= true)),
                    0xd9 => instr!(SetBit (= 3) (:R C) (= true)),
                    0xda => instr!(SetBit (= 3) (:R D) (= true)),
                    0xdb => instr!(SetBit (= 3) (:R E) (= true)),
                    0xdc => instr!(SetBit (= 3) (:R H) (= true)),
                    0xdd => instr!(SetBit (= 3) (:R L) (= true)),
                    0xde => instr!(SetBit (= 3) (@R HL) (= true)),
                    0xdf => instr!(SetBit (= 3) (:R A) (= true)),
                    0xe0 => instr!(SetBit (= 4) (:R B) (= true)),
                    0xe1 => instr!(SetBit (= 4) (:R C) (= true)),
                    0xe2 => instr!(SetBit (= 4) (:R D) (= true)),
                    0xe3 => instr!(SetBit (= 4) (:R E) (= true)),
                    0xe4 => instr!(SetBit (= 4) (:R H) (= true)),
                    0xe5 => instr!(SetBit (= 4) (:R L) (= true)),
                    0xe6 => instr!(SetBit (= 4) (@R HL) (= true)),
                    0xe7 => instr!(SetBit (= 4) (:R A) (= true)),
                    0xe8 => instr!(SetBit (= 5) (:R B) (= true)),
                    0xe9 => instr!(SetBit (= 5) (:R C) (= true)),
                    0xea => instr!(SetBit (= 5) (:R D) (= true)),
                    0xeb => instr!(SetBit (= 5) (:R E) (= true)),
                    0xec => instr!(SetBit (= 5) (:R H) (= true)),
                    0xed => instr!(SetBit (= 5) (:R L) (= true)),
                    0xee => instr!(SetBit (= 5) (@R HL) (= true)),
                    0xef => instr!(SetBit (= 5) (:R A) (= true)),
                    0xf0 => instr!(SetBit (= 6) (:R B) (= true)),
                    0xf1 => instr!(SetBit (= 6) (:R C) (= true)),
                    0xf2 => instr!(SetBit (= 6) (:R D) (= true)),
                    0xf3 => instr!(SetBit (= 6) (:R E) (= true)),
                    0xf4 => instr!(SetBit (= 6) (:R H) (= true)),
                    0xf5 => instr!(SetBit (= 6) (:R L) (= true)),
                    0xf6 => instr!(SetBit (= 6) (@R HL) (= true)),
                    0xf7 => instr!(SetBit (= 6) (:R A) (= true)),
                    0xf8 => instr!(SetBit (= 7) (:R B) (= true)),
                    0xf9 => instr!(SetBit (= 7) (:R C) (= true)),
                    0xfa => instr!(SetBit (= 7) (:R D) (= true)),
                    0xfb => instr!(SetBit (= 7) (:R E) (= true)),
                    0xfc => instr!(SetBit (= 7) (:R H) (= true)),
                    0xfd => instr!(SetBit (= 7) (:R L) (= true)),
                    0xfe => instr!(SetBit (= 7) (@R HL) (= true)),
                    0xff => instr!(SetBit (= 7) (:R A) (= true)),
                }
            }
            0xcc => instr!(CallIf (F Zero) (= true) ABS16),
            0xcd => instr!(Call ABS16),
            0xce => instr!(Add8 (R A) IMM8 (= true)),
            0xcf => instr!(Rst (= 1)),
            0xd0 => instr!(ReturnIf (F Carry) (= false)),
            0xd1 => instr!(Pop (R DE)),
            0xd2 => instr!(JumpIf (F Carry) (= false) ABS16),
            0xd4 => instr!(CallIf (F Carry) (= false) ABS16),
            0xd5 => instr!(Push (R DE)),
            0xd6 => instr!(Subtract IMM8 (= false)),
            0xd7 => instr!(Rst (= 2)),
            0xd8 => instr!(ReturnIf (F Carry) (= true)),
            0xd9 => instr!(ReturnInterrupt),
            0xda => instr!(JumpIf (F Carry) (= true) ABS16),
            0xdc => instr!(CallIf (F Carry) (= true) ABS16),
            0xde => instr!(Subtract IMM8 (= true)),
            0xdf => instr!(Rst (= 3)),
            0xe0 => instr!(Load (@IMM8 0xff00) (:R A)),
            0xe1 => instr!(Pop (R HL)),
            0xe2 => instr!(Load (@R C 0xff00) (:R A)),
            0xe5 => instr!(Push (R HL)),
            0xe6 => instr!(And IMM8),
            0xe7 => instr!(Rst (= 4)),
            0xe8 => instr!(SPOps (= SPOps::AddOffset(self.fetch_u8(mem)? as i8))),
            0xe9 => instr!(Jump (:R HL)),
            0xea => instr!(Load (@IMM16) (:R A)),
            0xee => instr!(Xor IMM8),
            0xef => instr!(Rst (= 5)),
            0xf0 => instr!(Load (:R A) (@IMM8 0xff00)),
            0xf1 => instr!(Pop (R AF)),
            0xf2 => instr!(Load (:R A) (@R C 0xff00)),
            0xf3 => instr!(DisableInterrupts),
            0xf5 => instr!(Push (R AF)),
            0xf6 => instr!(Or IMM8),
            0xf7 => instr!(Rst (= 6)),
            0xf8 => instr!(SPOps (= SPOps::LoadIntoHL(self.fetch_u8(mem)? as i8))),
            0xf9 => instr!(SPOps (= SPOps::LoadFromHL)),
            0xfa => instr!(Load (:R A) (@IMM16)),
            0xfb => instr!(EnableInterrupts),
            0xfe => instr!(Compare IMM8),
            0xff => instr!(Rst (= 7)),
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

    pub fn process_interrupts<M: Memory>(
        &mut self,
        mem: &mut M,
        interrupts: Interrupts,
    ) -> (usize, Interrupts) {
        let mut processed_interrupts = Interrupts::empty();

        if let InterruptState::Enabled = self.interrupt_state {
            let address = if interrupts.contains(Interrupts::VBLANK) {
                processed_interrupts.insert(Interrupts::VBLANK);
                0x40
            } else if interrupts.contains(Interrupts::LCD_STAT) {
                processed_interrupts.insert(Interrupts::LCD_STAT);
                0x48
            } else if interrupts.contains(Interrupts::TIMER) {
                processed_interrupts.insert(Interrupts::TIMER);
                0x50
            } else if interrupts.contains(Interrupts::SERIAL) {
                processed_interrupts.insert(Interrupts::SERIAL);
                0x58
            } else if interrupts.contains(Interrupts::JOYPAD) {
                processed_interrupts.insert(Interrupts::JOYPAD);
                0x60
            } else {
                return (0, processed_interrupts);
            };

            self.push_u16(mem, self.pc)
                .context("error while pushing interrupt return address")
                .unwrap();
            self.pc = address;
            self.interrupt_state = InterruptState::Disabled;

            return (5, processed_interrupts);
        }

        (0, processed_interrupts)
    }

    pub fn disassemble<M: Memory>(&mut self, mem: &mut M, max: u16) -> BTreeMap<u16, String> {
        let old_pc = self.pc;
        let mut res = BTreeMap::new();

        self.pc = 0;
        let mut pc = 0;
        while !res.contains_key(&pc) && pc < max {
            let instruction = self.fetch_instruction(mem);
            if let Ok(instruction) = instruction {
                res.insert(pc, format!("{:#06x}: {}", pc, instruction));
            } else {
                res.insert(pc, format!("{:#06x}: <unknown>", pc));
            }
            pc = self.pc;
        }

        self.pc = old_pc;

        res
    }
}
