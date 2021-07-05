use std::fmt;

use thiserror::Error;

pub mod mmu;

#[derive(Debug, Clone, Copy)]
pub enum MemoryOperation {
    Read,
    Write,
}

impl fmt::Display for MemoryOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryOperation::Read => write!(f, "read"),
            MemoryOperation::Write => write!(f, "write"),
        }
    }
}

#[derive(Error, Debug, Clone, Copy)]
pub enum MemoryError {
    #[error("trying to access unmapped memory at {address:#06x}")]
    Unmapped { address: u16 },
    #[error("illegal {op} to memory at {address:#06x}")]
    Illegal { address: u16, op: MemoryOperation },
    #[error("write to read-only memory at {address:#06x}")]
    ReadOnly { address: u16 },
}

pub trait Memory {
    fn read(&self, address: u16) -> Result<u8, MemoryError>;
    fn write(&mut self, address: u16, value: u8) -> Result<(), MemoryError>;
}
