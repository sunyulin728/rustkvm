#![allow(dead_code)]
#![allow(non_snake_case)]
#![feature(proc_macro_hygiene, asm)]

extern crate alloc;
extern crate spin;
extern crate x86_64;

mod kvmlib;

fn main() {
    use kvmlib::KVMMachine;
    //use kvmlib::ELFLoader::Loader;

    //let elf = Loader::Init(&String::from("/home/brad/rust/quark/qkernel/build/kernel-x86_64.bin")).expect("asdf");
    //println!("{:?}", elf);

    //kvmlib::ELFLoader::elftest();

    match KVMMachine::init(0x200000) {
        Ok(mut vm) => {
            println!("test....");

            //vm.MapMemRange(kvmlib::Addr::Addr(vm.mem as u64), 4096, kvmlib::Addr::Addr(0)).expect("asdf");
            //println!("start to run*************");
            vm.run().expect("asdf1111")
        },
        Err(e) => println!("error is {:?}", e)
    }
}
