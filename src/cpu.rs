use std::u8;

use crate::{
    instruction::{CpuRegister, Instruction, InstructionOperand},
    mmu::Mmu,
};

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

    pub fn pop_u16(&mut self, mmu: &mut Mmu) -> u16 {
        let lo = mmu.read(self.sp);
        self.sp = self.sp.wrapping_add(1);
        let hi = mmu.read(self.sp);
        self.sp = self.sp.wrapping_add(1);

        (hi as u16) << 8 | (lo as u16)
    }

    pub fn push_u16(&mut self, mmu: &mut Mmu, value: u16) {
        let hi = (value >> 8) as u8;
        let lo = value as u8;

        self.sp = self.sp.wrapping_sub(1);
        mmu.write(self.sp, hi);
        self.sp = self.sp.wrapping_sub(1);
        mmu.write(self.sp, lo);
    }

    fn get_reg_u8(&mut self, reg: CpuRegister) -> u8 {
        match reg {
            CpuRegister::A => self.a,
            CpuRegister::B => self.b,
            CpuRegister::C => self.c,
            CpuRegister::D => self.d,
            CpuRegister::E => self.e,
            CpuRegister::H => self.h,
            CpuRegister::L => self.l,
            CpuRegister::F => self.f,
            _ => panic!("tried to get u8 value of register {:?}", reg),
        }
    }

    fn set_reg_u8(&mut self, reg: CpuRegister, value: u8) {
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
    }

    fn get_reg_u16(&mut self, reg: CpuRegister) -> u16 {
        match reg {
            CpuRegister::A => self.a as u16,
            CpuRegister::B => self.b as u16,
            CpuRegister::C => self.c as u16,
            CpuRegister::D => self.d as u16,
            CpuRegister::E => self.e as u16,
            CpuRegister::H => self.h as u16,
            CpuRegister::L => self.l as u16,
            CpuRegister::F => self.f as u16,
            CpuRegister::AF => self.af(),
            CpuRegister::BC => self.bc(),
            CpuRegister::DE => self.de(),
            CpuRegister::HL => self.hl(),
            CpuRegister::SP => self.sp,
        }
    }

    fn set_reg_u16(&mut self, reg: CpuRegister, value: u16) {
        match reg {
            CpuRegister::AF => self.set_af(value),
            CpuRegister::BC => self.set_bc(value),
            CpuRegister::DE => self.set_de(value),
            CpuRegister::HL => self.set_hl(value),
            CpuRegister::SP => self.sp = value,
            _ => panic!("tried to set u16 value of register {:?}", reg),
        }
    }

    fn get_u8(&mut self, mmu: &mut Mmu, operand: InstructionOperand) -> u8 {
        match operand {
            InstructionOperand::Register(reg) => self.get_reg_u8(reg),
            InstructionOperand::Immediate8(val) => val,
            InstructionOperand::OffsetMemoryLocationRegister(offset, reg) => {
                mmu.read(self.get_reg_u16(reg).wrapping_add(offset))
            }
            InstructionOperand::MemoryLocationRegister(reg) => mmu.read(self.get_reg_u16(reg)),
            InstructionOperand::MemoryLocationRegisterDecrement(reg) => {
                let value = mmu.read(self.get_reg_u16(reg));
                let reg_value = self.get_reg_u16(reg).wrapping_sub(1);
                self.set_reg_u16(reg, reg_value);
                value
            }
            InstructionOperand::MemoryLocationRegisterIncrement(reg) => {
                let value = mmu.read(self.get_reg_u16(reg));
                let reg_value = self.get_reg_u16(reg).wrapping_add(1);
                self.set_reg_u16(reg, reg_value);
                value
            }
            InstructionOperand::MemoryLocationImmediate16(address) => mmu.read(address),
            _ => panic!("tried to get u8 value of {:?}", &operand),
        }
    }

    fn set_u8(&mut self, mmu: &mut Mmu, operand: InstructionOperand, value: u8) {
        match operand {
            InstructionOperand::Register(reg) => self.set_reg_u8(reg, value),
            InstructionOperand::OffsetMemoryLocationRegister(offset, reg) => {
                mmu.write(self.get_reg_u16(reg).wrapping_add(offset), value)
            }
            InstructionOperand::MemoryLocationRegister(reg) => {
                mmu.write(self.get_reg_u16(reg), value)
            }
            InstructionOperand::MemoryLocationRegisterDecrement(reg) => {
                mmu.write(self.get_reg_u16(reg), value);
                let reg_value = self.get_reg_u16(reg).wrapping_sub(1);
                self.set_reg_u16(reg, reg_value);
            }
            InstructionOperand::MemoryLocationRegisterIncrement(reg) => {
                mmu.write(self.get_reg_u16(reg), value);
                let reg_value = self.get_reg_u16(reg).wrapping_add(1);
                self.set_reg_u16(reg, reg_value);
            }
            InstructionOperand::MemoryLocationImmediate16(address) => mmu.write(address, value),
            _ => panic!("tried to set u8 value of {:?}", &operand),
        }
    }

    fn get_u16(&mut self, mmu: &mut Mmu, operand: InstructionOperand) -> u16 {
        match operand {
            InstructionOperand::Register(reg) => self.get_reg_u16(reg),
            InstructionOperand::Immediate8(val) => val as u16,
            InstructionOperand::Immediate16(val) => val,
            InstructionOperand::OffsetMemoryLocationRegister(offset, reg) => {
                mmu.read(self.get_reg_u16(reg).wrapping_add(offset)) as u16
            }
            InstructionOperand::MemoryLocationRegister(reg) => {
                mmu.read(self.get_reg_u16(reg)) as u16
            }
            InstructionOperand::MemoryLocationRegisterDecrement(reg) => {
                let value = mmu.read(self.get_reg_u16(reg)) as u16;
                let reg_value = self.get_reg_u16(reg).wrapping_sub(1);
                self.set_reg_u16(reg, reg_value);
                value
            }
            InstructionOperand::MemoryLocationRegisterIncrement(reg) => {
                let value = mmu.read(self.get_reg_u16(reg)) as u16;
                let reg_value = self.get_reg_u16(reg).wrapping_add(1);
                self.set_reg_u16(reg, reg_value);
                value
            }
            InstructionOperand::MemoryLocationImmediate16(address) => mmu.read(address) as u16,
        }
    }

    fn set_u16(&mut self, operand: InstructionOperand, value: u16) {
        match operand {
            InstructionOperand::Register(reg) => self.set_reg_u16(reg, value),
            _ => panic!("tried to set u16 value of {:?}", &operand),
        }
    }

    pub fn exec_next_instruction(&mut self, mmu: &mut Mmu) -> usize {
        let instruction = self.fetch_instruction(mmu).unwrap();
        self.exec_instruction(mmu, instruction)
    }

    pub fn exec_instruction(&mut self, mmu: &mut Mmu, instruction: Instruction) -> usize {
        let mut cycles = instruction.cycles();

        match instruction {
            Instruction::Noop => {}
            Instruction::Load(to, from) => {
                if to.is_16bit() {
                    let val = self.get_u16(mmu, from);
                    self.set_u16(to, val);
                } else {
                    let val = self.get_u8(mmu, from);
                    self.set_u8(mmu, to, val);
                }
            }
            Instruction::Xor(from) => {
                self.a = self.a ^ self.get_u8(mmu, from);

                self.set_flag(CpuFlag::Zero, self.a == 0);
                self.set_flag(CpuFlag::Subtraction, false);
                self.set_flag(CpuFlag::HalfCarry, false);
                self.set_flag(CpuFlag::Carry, false);
            }
            Instruction::Bit(bit, from) => {
                let set = self.get_u8(mmu, from) & (1 << bit) != 0;

                self.set_flag(CpuFlag::Zero, set);
                self.set_flag(CpuFlag::Subtraction, false);
                self.set_flag(CpuFlag::HalfCarry, true);
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
                    let val = self.get_u16(mmu, to).wrapping_add(1);
                    self.set_u16(to, val);
                } else {
                    let val = self.get_u8(mmu, to).wrapping_add(1);
                    self.set_u8(mmu, to, val);

                    self.set_flag(CpuFlag::Zero, val == 0);
                    self.set_flag(CpuFlag::Subtraction, false);
                    self.set_flag(CpuFlag::HalfCarry, val & 0x10 != 0);
                }
            }
            Instruction::Decrement(to) => {
                if to.is_16bit() {
                    let val = self.get_u16(mmu, to).wrapping_sub(1);
                    self.set_u16(to, val);
                } else {
                    let val = self.get_u8(mmu, to).wrapping_sub(1);
                    self.set_u8(mmu, to, val);

                    self.set_flag(CpuFlag::Zero, val == 0);
                    self.set_flag(CpuFlag::Subtraction, true);
                    self.set_flag(CpuFlag::HalfCarry, val & 0x10 != 0);
                }
            }
            Instruction::Call(address) => {
                self.push_u16(mmu, self.pc);
                self.pc = address;
            }
            Instruction::Push(reg) => {
                let value = self.get_reg_u16(reg);
                self.push_u16(mmu, value)
            }
            Instruction::Pop(reg) => {
                let value = self.pop_u16(mmu);
                self.set_reg_u16(reg, value);
            }
            Instruction::ExtendedRotateLeft(to) => {
                let carry = self.get_flag(CpuFlag::Carry) as u8;
                let previous = self.get_u8(mmu, to);

                self.set_flag(CpuFlag::Carry, previous & 0x80 != 0);

                let value = previous << 1 | carry;
                self.set_u8(mmu, to, value);

                self.set_flag(CpuFlag::Zero, value == 0);
                self.set_flag(CpuFlag::Subtraction, false);
                self.set_flag(CpuFlag::HalfCarry, false);
            }
            Instruction::RotateLeftA => {
                let carry = self.get_flag(CpuFlag::Carry) as u8;
                self.set_flag(CpuFlag::Carry, self.a & 0x80 != 0);
                self.a = self.a << 1 | carry;
            }
            Instruction::Return => self.pc = self.pop_u16(mmu),
            Instruction::Compare(to) => {
                let value = self.get_u8(mmu, to);
                self.set_flag(CpuFlag::Zero, value == self.a);
            }
            _ => panic!("unimplemented instruction {:x?}", instruction),
        }

        cycles
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
            0x17 => Some(Instruction::RotateLeftA),
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
            0x22 => Some(Instruction::Load(
                InstructionOperand::MemoryLocationRegisterIncrement(CpuRegister::HL),
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
            0x32 => Some(Instruction::Load(
                InstructionOperand::MemoryLocationRegisterDecrement(CpuRegister::HL),
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
            0x11 => Some(Instruction::ExtendedRotateLeft(
                InstructionOperand::Register(CpuRegister::C),
            )),
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
