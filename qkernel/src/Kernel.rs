use spin::Mutex;
use lazy_static::lazy_static;
use super::qlib;
use alloc::string::String;

lazy_static! {
    pub static ref KERNEL: Mutex<Kernel> = Mutex::new(Kernel::Init());
}

pub struct Kernel {

}

impl Kernel {
    pub fn Init() -> Self {
        return Kernel{}
    }

    pub fn Print(str: &str) {
        let mut msg = qlib::Msg::Print (String::from(str));
        Kernel::Call(&mut msg);
    }

    fn Call(event: &mut qlib::Msg) {
        super::SHARESPACE.lock().SetMsg(event);
        qlib::Wait();
    }
}


