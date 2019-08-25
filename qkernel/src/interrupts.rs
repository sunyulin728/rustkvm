use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        idt.divide_by_zero.set_handler_fn(breakpoint_handler);
        idt.non_maskable_interrupt.set_handler_fn(breakpoint_handler);
        idt.debug.set_handler_fn(breakpoint_handler);
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.overflow.set_handler_fn(breakpoint_handler);
        idt.bound_range_exceeded.set_handler_fn(breakpoint_handler);
        idt.invalid_opcode.set_handler_fn(breakpoint_handler);
        idt.device_not_available.set_handler_fn(breakpoint_handler);

        idt.double_fault.set_handler_fn(double_fault_handler);

        idt.invalid_tss.set_handler_fn(double_fault_handler);
        idt.segment_not_present.set_handler_fn(double_fault_handler);
        idt.stack_segment_fault.set_handler_fn(double_fault_handler);
        idt.general_protection_fault.set_handler_fn(double_fault_handler);
        //idt.page_fault.set_handler_fn(breakpoint_handler);
        idt.x87_floating_point.set_handler_fn(breakpoint_handler);
        idt.alignment_check.set_handler_fn(double_fault_handler);
        idt.machine_check.set_handler_fn(breakpoint_handler);
        idt.simd_floating_point.set_handler_fn(breakpoint_handler);
        idt.virtualization.set_handler_fn(breakpoint_handler);
        idt.security_exception.set_handler_fn(double_fault_handler);

        /*
        unsafe {
            idt.breakpoint.set_handler_fn(breakpoint_handler).set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);;
        }



        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }*/
        idt
    };
}

pub fn init_idt() {
    use core::mem::size_of;

    let idtAddr : u64 = &IDT as *const _ as u64;
    let limit = (size_of::<InterruptDescriptorTable>() - 1) as u16;
    kprintln!("the idt addr is {:x}, limit={:x}", idtAddr, limit);

    //IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    println!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}