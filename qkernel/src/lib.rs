//#![feature(macro_rules)]
#![feature(lang_items)]
#![no_std]
#![feature(proc_macro_hygiene, asm)]
#![feature(alloc_error_handler)]
#![feature(abi_x86_interrupt)]
#![allow(dead_code)]
#![allow(non_snake_case)]
#![feature(naked_functions)]
//#![feature(const_raw_ptr_to_usize_cast)]

extern crate rusty_asm;
extern crate alloc;
extern crate spin;
extern crate linked_list_allocator;
extern crate lazy_static;
extern crate x86_64;

mod print;
mod qlib;
mod interrupts;
mod Kernel;

use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;
use qlib::{ShareSpace};
use lazy_static::lazy_static;
use spin::Mutex;

pub const HEAP_START: usize = 0x70_2000_0000;
pub const HEAP_SIZE: usize = 0x1000_0000;

#[global_allocator]
static ALLOCATOR:  LockedHeap = LockedHeap::empty();

lazy_static! {
    pub static ref SHARESPACE: Mutex<ShareSpace> = Mutex::new(ShareSpace::Init());
}

#[no_mangle]
pub extern fn rust_main() {
    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    qlib::HyperCall(qlib::HYPERCALL_INIT, (&(*SHARESPACE) as * const Mutex<ShareSpace>) as u64);

    interrupts::init_idt();

    for i in 0..10 {
        println!("in kernel {}", i);
    }

    //enable to repro the exception issue
    //x86_64::instructions::interrupts::int3();

    println!("in kernel end....");
}
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    qlib::Out(qlib::HYPERCALL_PANIC, 0);
    loop {}
}

#[alloc_error_handler]
fn alloc_error_handler(_layout: alloc::alloc::Layout) -> ! {
    qlib::Out(qlib::HYPERCALL_PANIC, 0);
    loop {}
}

#[lang = "eh_personality"] extern fn eh_personality() {}


