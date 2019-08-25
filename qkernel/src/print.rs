#![macro_use]
use core::fmt;
use core::fmt::Write;
use core::fmt::Arguments;
use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref PRINT_STRUCT: Mutex<PrintStruct> = Mutex::new(PrintStruct{});
}

pub struct PrintStruct {

}

impl fmt::Write for PrintStruct {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        super::Kernel::Kernel::Print(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::print::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: Arguments) {
    PRINT_STRUCT.lock().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => ($crate::print::k_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! kprintln {
    () => ($crate::kprint!("\n"));
    ($($arg:tt)*) => ($crate::kprint!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn k_print(args: Arguments) {
    PRINT_STRUCT.lock().write_fmt(args).unwrap();
}