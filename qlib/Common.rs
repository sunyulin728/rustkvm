extern crate alloc;

use alloc::string::String;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub enum Error {
    None,
    Common(String),
    CreateMMap(String),
    UnallignedAddress,
    UnallignedSize,
    NoEnoughMemory,
    AddressNotInRange,
    RootPageIdxNoExist,
    IOError(String),
    NoEnoughSpace,
    RangeUnavailable,
    Overflow,
    WrongELFFormat,
    ELFLoadError(&'static str),
    InterpreterFileErr,
    MMampError,
    UnmatchRegion,
    AddressDoesMatch,
    Locked,
    ZeroCount,
    QueueFull,
    NoData,
    NoneIdx,
    AddressNotMap,
}

impl Default for Error {
    fn default() -> Self { Error::None }
}