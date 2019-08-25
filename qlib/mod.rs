#![macro_use]
extern crate rusty_asm;
extern crate alloc;
extern crate spin;

pub mod Common;
pub mod Addr;
pub mod PageTable;

use alloc::string::String;

pub const HYPERCALL_INIT : u16 = 1;
pub const HYPERCALL_PANIC : u16 = 2;
pub const HYPERCALL_WAIT : u16 = 3;
pub const HYPERCALL_LOADIDT : u16 = 4;

const MSG_QLEN: u32 = 1024;
const MSG_INIT_COUNT: u32 = 8;

pub const ONE_MB: u64 = 0x100_000;
pub const ONE_GB: u64 = 0x40_000_000;
pub const ONE_TB:  u64 = 0x1_000_000_000; //0x10_000_000_000;

pub const DEFAULT_STACK_SIZE : u64 = 2 * ONE_MB;  //2MB
pub const PAGE_SIZE : u64 = 0x1000;
pub const KERNEL_BASE_ADDR: u64 = 7 * ONE_TB;
pub const KERNEL_ADDR_SIZE: u64 = 128 * ONE_TB;
pub const PHY_MEM_SPACE : u64 = 8*ONE_TB;
pub const PAGE_MASK : u64 = PAGE_SIZE-1;

pub const PAGE_SIZE_4K : u64 = 0x1000;
pub const PAGE_SIZE_2M : u64 = (2*ONE_MB);
pub const STACK_SIZE : u64 = 6 * ONE_MB;
pub const STACK_GUARDPAGE_SIZE : u64 = 2 * ONE_MB;
pub const BLOCK_SIZE:  u64 = 64 * ONE_GB;
pub const PHY_UPPER_ADDR: u64 = 7 * BLOCK_SIZE;
pub const LOWER_TOP : u64 = 0x00007fffffffffff;
pub const UPPER_BOTTOM : u64 = 0xffff800000000000;
pub const ENTRY_COUNT: u16 = 512 as u16;

pub fn HyperCall(type_: u16, para1: u64) {
    unsafe {
        rusty_asm::rusty_asm! {
            let port: in("{dx}") = type_;
            let value: in("{eax}") = 0 as u32;
            let addr3: in("{rcx}") = para1;
            asm("volatile", "intel") {
                "out $port, $value"
            }
        }
    }
}

pub fn Out(port: u16, value: u32) {
    unsafe {
        rusty_asm::rusty_asm! {
            let port: in("{dx}") = port;
            let value: in("{eax}") = value as u32;
            asm("volatile", "intel") {
                "out $port, $value"
            }
        }
    }
}

pub fn Hlt() {
    unsafe {
        rusty_asm::rusty_asm! {
           asm("volatile", "intel") {
                "hlt"
            }
        }
    }
}

//Kernel thread wait for ready task
pub fn Wait() {
    HyperCall(HYPERCALL_WAIT, 0)
}

#[derive(Clone, Default, Debug)]
pub struct Str {
    pub addr: u64,
    pub len: u32
}

#[derive(Clone, Debug)]
pub enum Msg {
    Print (String),
    Msg1 { addr: u64, len: u32}
}

pub struct ShareSpace {
    pub msgAddr : u64
}

impl ShareSpace {
    pub fn Init() -> Self {
        ShareSpace { msgAddr : 0}
    }

    pub fn SetMsg(&mut self, msg: &mut Msg) {
        self.msgAddr = &(*msg) as *const _ as u64;
    }

    pub fn GetMsg(&mut self) -> u64 {
        return self.msgAddr
    }
}
