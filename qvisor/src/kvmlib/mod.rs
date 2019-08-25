#![allow(dead_code)]
#![allow(non_snake_case)]

extern crate alloc;

mod qlib;
mod MemMgr;
mod vmspace;

pub mod ELFLoader;

use std::sync::Arc;
use std::cell::RefCell;

use qlib::Common::Error;
use qlib::Common::Result;

use qlib::{ShareSpace, Addr};
use spin::Mutex;

use std::boxed::Box;

//use kvm_bindings::KVM_MEM_LOG_DIRTY_PAGES;
use kvm_bindings::kvm_userspace_memory_region;
use kvm_bindings::kvm_sregs;
use kvm_bindings::kvm_segment;
use kvm_bindings::kvm_regs;
//use kvm_bindings::{kvm_fpu, kvm_msr_entry, kvm_msrs, kvm_regs, kvm_sregs};
use kvm_bindings::KVM_MEM_LOG_DIRTY_PAGES;
//use MemMgr::MemSpaceMgr;

//use kvm_ioctls::{Kvm, VmFd, VcpuFd};
use kvm_ioctls::{Kvm, VmFd};
use kvm_ioctls::VcpuExit;

use qlib::PageTable::{PageTables,PagePool};
use MemMgr::MappedRegion;
use MemMgr::PhyAddrMgr;
use MemMgr::MapOption;
use ELFLoader::KernelELF;
use lazy_static::lazy_static;

// copy include/uapi/linux/if_tun.h from the kernel code.
const KVM_GET_API_VERSION: u64 = 0xae00;
const KVM_CREATE_VM: u64 = 0xae01;
const KVM_CHECK_EXTENSION: u64 = 0xae03;
const KVM_GET_VCPU_MMAP_SIZE: u64 = 0xae04;
const KVM_CREATE_VCPU: u64 = 0xae41;
const KVM_GET_DIRTY_LOG: u64 = 0x4010_ae42;
const KVM_SET_TSS_ADDR: u64 = 0xae47;
const KVM_CREATE_IRQCHIP: u64 = 0xae60;
const KVM_RUN: u64 = 0xae80;
const KVM_SET_MSRS: u64 = 0x4008_ae89;
const KVM_SET_CPUID2: u64 = 0x4008_ae90;
const KVM_SET_USER_MEMORY_REGION: u64 = 0x4020_ae46;
const KVM_IRQFD: u64 = 0x4020_ae76;
const KVM_CREATE_PIT2: u64 = 0x4040_ae77;
const KVM_IOEVENTFD: u64 = 0x4040_ae79;
const KVM_SET_REGS: u64 = 0x4090_ae82;
const KVM_SET_SREGS: u64 = 0x4138_ae84;
const KVM_SET_FPU: u64 = 0x41a0_ae8d;
const KVM_SET_LAPIC: u64 = 0x4400_ae8f;
const KVM_GET_SREGS: u64 = 0x8138_ae83;
const KVM_GET_LAPIC: u64 = 0x8400_ae8e;
const KVM_GET_SUPPORTED_CPUID: u64 = 0xc008_ae05;

const CR0_PE: u64 =  1;
const CR0_MP: u64 =  (1 << 1);
const CR0_EM: u64 =  (1 << 2);
const CR0_TS: u64 =  (1 << 3);
const CR0_ET: u64 =  (1 << 4);
const CR0_NE: u64 =  (1 << 5);
const CR0_WP: u64 =  (1 << 16);
const CR0_AM: u64 =  (1 << 18);
const CR0_NW: u64 =  (1 << 29);
const CR0_CD: u64 =  (1 << 30);
const CR0_PG: u64 =  (1 << 31);

const CR4_VME: u64 =  1;
const CR4_PVI: u64 =  (1 << 1);
const CR4_TSD: u64 =  (1 << 2);
const CR4_DE: u64 =  (1 << 3);
const CR4_PSE: u64 =  (1 << 4);
const CR4_PAE: u64 =  (1 << 5);
const CR4_MCE: u64 =  (1 << 6);
const CR4_PGE: u64 =  (1 << 7);
const CR4_PCE: u64 =  (1 << 8);
const CR4_OSFXSR: u64 =  (1 << 8);
const CR4_OSXMMEXCPT: u64 =  (1 << 10);
const CR4_UMIP: u64 =  (1 << 11);
const CR4_VMXE: u64 =  (1 << 13);
const CR4_SMXE: u64 =  (1 << 14);
const CR4_FSGSBASE: u64 =  (1 << 16);
const CR4_PCIDE: u64 =  (1 << 17);
const CR4_OSXSAVE: u64 =  (1 << 18);
const CR4_SMEP: u64 =  (1 << 20);
const CR4_SMAP: u64 =  (1 << 21);
const CR4_PKE: u64 =  (1 << 22);

const EFER_SCE: u64 =  1;
const EFER_LME: u64 =  (1 << 8);
const EFER_LMA: u64 =  (1 << 10);
const EFER_NXE: u64 =  (1 << 11);
const EFER_SVME: u64 =  (1 << 12);
const EFER_LMSLE: u64 =  (1 << 13);
const EFER_FFXSR: u64 =  (1 << 14);
const EFER_TCE: u64 =  (1 << 15);

// 32-bit page directory entry bits
const PDE32_PRESENT: u64 =  1;
const PDE32_RW: u64 =  (1 << 1);
const PDE32_USER: u64 =  (1 << 2);
const PDE32_PS: u64 =  (1 << 7);

const PDE64_PRESENT : u64 = 1;
const PDE64_RW : u64 = (1 << 1);
const PDE64_USER : u64 = (1 << 2);
const PDE64_ACCESSED : u64 = (1 << 5);
const PDE64_DIRTY : u64 = (1 << 6);
const PDE64_PS : u64 = (1 << 7);
const PDE64_G : u64 = (1 << 8);

const LOWER_TOP : u64 = 0x00007fffffffffff;
const UPPER_BOTTOM : u64 = 0xffff800000000000;

lazy_static! {
    pub static ref VMS: Mutex<vmspace::VMSpace> = Mutex::new(vmspace::VMSpace::default());
}

pub struct KVMMachine {
    pub kvm : kvm_ioctls::Kvm,
    pub vm_fd: kvm_ioctls::VmFd,
    pub vcpu_fds :  Vec<kvm_ioctls::VcpuFd>,

    pub pageMmap: Box<MappedRegion>,
    pub phyAddrMgr : Arc<RefCell<PhyAddrMgr>>,

    pub topStackAddr: u64,
    pub defaultStackAddr: u64,
    pub entry: u64,

    pub elf: KernelELF,
}

impl KVMMachine {
    fn initKernelMem(_vm_fd : &VmFd, phyUpperAddr:u64, pageMmapsize: u64) -> Result<Box<MappedRegion>> {
        //let vmSpace : vmspace::VMSpace;

        let mr = MapOption::New().Offset(phyUpperAddr).Len(pageMmapsize).MapHugeTLB().MapAnan().MapPrivate().ProtoRead().ProtoWrite().ProtoExec().Map()?;
        let pageMmap = Box::new(mr);

        //let pageMmap = Box::new(MappedRegion::Init(Addr::Addr(phyUpperAddr), pageMmapsize, true, libc::PROT_READ |  libc::PROT_WRITE |  libc::PROT_EXEC)?);

        let pageMapAddr = pageMmap.as_ptr() as *mut u8;
        if pageMapAddr as u64 != phyUpperAddr {
            return Err(Error::AddressDoesMatch);
        }

        return Ok(pageMmap)
    }

    pub fn SetMemRegion(slotId: u32, vm_fd : &VmFd, phyUpperAddr:u64, pageMmapsize: u64) -> Result<()> {
        println!("SetMemRegion phyUpperAddr = {:x}, pageMmapsize = {:x}", phyUpperAddr, pageMmapsize);

        let mem_region = kvm_userspace_memory_region {
            slot: slotId,
            guest_phys_addr: phyUpperAddr,
            memory_size: pageMmapsize,
            userspace_addr: phyUpperAddr,
            flags: KVM_MEM_LOG_DIRTY_PAGES,
        };
        vm_fd.set_user_memory_region(mem_region).map_err(|e| Error::IOError(format!("io::error is {:?}", e)))?;
        return Ok(())
    }

    pub fn init(_mem_size:usize) -> Result<Self> {
        let kvm = Kvm::new().map_err(|e| Error::IOError(format!("io::error is {:?}", e)))?;
        let vm_fd = kvm.create_vm().map_err(|e| Error::IOError(format!("io::error is {:?}", e)))?;


        let mut elf = KernelELF::Init(&String::from("/home/brad/rust/rustkvm/qkernel/build/kernel-x86_64.bin"))?;

        //PageMem: 512MB; HeapSize: 512MB; Stack:128MB (16 x (2MB guard page + 6MB stack size))

        let kernelMemSize = 1 * MemMgr::ONE_GB + 128 * MemMgr::ONE_MB;

        if elf.StartAddr().0 != MemMgr::PHY_UPPER_ADDR + kernelMemSize {
            return Err(Error::AddressDoesMatch)
        }

        let pageMmap = KVMMachine::initKernelMem(&vm_fd, MemMgr::PHY_UPPER_ADDR, kernelMemSize)?;

        //KVMMachine::SetMemRegion(&vm_fd, MemMgr::PHY_UPPER_ADDR, elf.EndAddr().0 - MemMgr::PHY_UPPER_ADDR)?;
        //KVMMachine::SetMemRegion(0, &vm_fd, MemMgr::PHY_UPPER_ADDR, 16 * MemMgr::ONE_GB)?;

        println!("the end address is {:x}", elf.EndAddr().0);
        for i in 0..1 {
            KVMMachine::SetMemRegion(i, &vm_fd, MemMgr::PHY_UPPER_ADDR + i as u64 * 16 * MemMgr::ONE_GB, 16 * MemMgr::ONE_GB)?;
        }

        println!("set map ragion start={:x}, end={:x}", MemMgr::PHY_UPPER_ADDR, MemMgr::PHY_UPPER_ADDR + 16 * MemMgr::ONE_GB);

        let mut topStackAddr = pageMmap.as_ptr() as u64 + 1 * MemMgr::ONE_GB;
        let defaultStackAddr = topStackAddr + MemMgr::STACK_GUARDPAGE_SIZE + MemMgr::STACK_SIZE;

        {
            let vms =  &mut VMS.lock();
            vms.pagePool = Some(PagePool::Init(Addr::Addr(pageMmap.as_ptr() as u64), 256*1024)?);
            vms.pageTables = Some(PageTables::New(vms.pagePool.as_mut().unwrap())?);
            vms.Map(Addr::Addr(pageMmap.as_ptr() as u64), Addr::Addr(pageMmap.as_ptr() as u64 + 1 * MemMgr::ONE_GB), Addr::Addr(pageMmap.as_ptr() as u64),
                                             &Addr::PageOpts::Default())?;
            vms.Map(Addr::Addr(topStackAddr), Addr::Addr(defaultStackAddr), Addr::Addr(topStackAddr), &Addr::PageOpts::Default())?;
         }


        topStackAddr += MemMgr::STACK_GUARDPAGE_SIZE + MemMgr::STACK_SIZE;

         //todo: allocate stack with guard page one by one
        let hostMemOffset = Addr::Addr(pageMmap.as_ptr() as u64).AddLen(kernelMemSize)?;

        /*let len = 7 * MemMgr::BLOCK_SIZE;
        let mem_region = kvm_userspace_memory_region {
            slot : 1,
            guest_phys_addr: 0x0,
            memory_size: len, //7 * MemMgr::BLOCK_SIZE, //len,
            userspace_addr: hostMemOffset.0,
            flags: KVM_MEM_LOG_DIRTY_PAGES,
        };
        vm_fd.set_user_memory_region(mem_region).map_err(|e| Error::IOError(format!("io::error is {:?}", e)))?;

        println!("extra memory ragion start from {:x} to {:x}", hostMemOffset.0, hostMemOffset.0 +  len);*/

        let phyAddrMgr = Arc::new(RefCell::new(PhyAddrMgr::Init(hostMemOffset,  7 * MemMgr::BLOCK_SIZE)?));

        let entry = elf.LoadKernel()?;

        let p = entry as *const u8;
        println!("entry is 0x{:x}, data at entry is {:x}", entry, unsafe{*p} );

        Ok(KVMMachine {
            kvm: kvm,
            vm_fd : vm_fd,
            vcpu_fds: Vec::new(),
            pageMmap,
            topStackAddr,
            defaultStackAddr,
            entry: entry,
            phyAddrMgr,
            elf,
        })
    }

    fn CreateVCPU(&mut self) -> Result<()> {
        self.vcpu_fds.push(self.vm_fd.create_vcpu(0).map_err(|e| Error::IOError(format!("io::error is {:?}", e)))?);
        Ok(())
    }

    fn setup_64bit_code_segment(sregs : &mut kvm_sregs) {
        let cs_seg = kvm_segment {
            base : 0,
            limit : 0xffffffff,
            selector : 1 << 3,
            present : 1,
            type_ : 11, /* Code: execute, read, accessed */
            dpl : 0,
            db : 0,
            s : 1, /* Code/data */
            l : 1,
            g : 1, /* 4KB granularity */
            avl: 0,
            padding: 0,
            unusable: 0,
        };

        let ds_seg = kvm_segment{
            type_ : 3,
            selector : 2<<3,
            ..cs_seg
        };

        sregs.cs = cs_seg;
        sregs.ds = ds_seg;
        sregs.es = ds_seg.clone();
        sregs.fs = ds_seg.clone();
        sregs.gs = ds_seg.clone();
        sregs.ss = ds_seg.clone();
    }

    fn setup_long_mode(&self) -> Result<()> {

        let mut vcpu_sregs = self.vcpu_fds[0].get_sregs().map_err(|e| Error::IOError(format!("io::error is {:?}", e)))?;

        vcpu_sregs.cr3 = {VMS.lock().pageTables.as_ref().unwrap().root.0};
        vcpu_sregs.cr4 = CR4_PAE;
        vcpu_sregs.cr0 = CR0_PE | CR0_MP | CR0_ET | CR0_NE | CR0_WP | CR0_AM | CR0_PG;

        vcpu_sregs.efer = EFER_LME | EFER_LMA | EFER_SCE;

        KVMMachine::setup_64bit_code_segment(&mut vcpu_sregs);

        self.vcpu_fds[0].set_sregs(&vcpu_sregs).map_err(|e| Error::IOError(format!("io::error is {:?}", e)))?;
        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {



        self.CreateVCPU()?;
        self.setup_long_mode()?;

        //println!("***the first byte of code is {}", self.mmap[0xb0]);

        let regs : kvm_regs = kvm_regs {
            rflags: 2,
            rip: self.entry,
            rsp: self.topStackAddr-1,
            rax: 0x11,
            rbx: 0xdd,
            rdx: 0x123,
            ..Default::default()
        };

        println!("entry is {:x}, stack is {:x}", self.entry, self.topStackAddr);

        self.vcpu_fds[0].set_regs(&regs).map_err(|e| Error::IOError(format!("io::error is {:?}", e)))?;

        let mut shareSpace : * mut Mutex<ShareSpace> = 0 as * mut Mutex<ShareSpace>; //give a default to work around compile uninitialized error

        loop {
            match self.vcpu_fds[0].run().expect("run failed") {
                VcpuExit::IoIn(addr, data) => {
                    println!(
                        "Received an I/O in exit. Address: {:#x}. Data: {:#x}",
                        addr,
                        data[0],
                    );
                }
                VcpuExit::IoOut(addr, _data) => {

                    match addr {
                        qlib::HYPERCALL_INIT => {
                            println!("get io out: HYPERCALL_INIT");

                            let regs = self.vcpu_fds[0].get_regs().map_err(|e| Error::IOError(format!("io::error is {:?}", e)))?;
                            shareSpace = regs.rcx as * mut Mutex<ShareSpace>;
                        },


                        qlib::HYPERCALL_WAIT => {
                             unsafe {
                                let mut k = (*shareSpace).lock();
                                let addr = {(*k).GetMsg()};

                                let msg = addr as *mut qlib::Msg;
                                match &(*msg) {
                                    qlib::Msg::Msg1{addr:_, len:_} => {
                                        println!("get message$3")
                                    }
                                    qlib::Msg::Print(str) => {
                                        print!("{}", str);
                                    }
                                }

                            }

                        },

                        qlib::HYPERCALL_PANIC => {
                            println!("get pannic");
                        },

                        _ => println!("asdfsadfdasasdfasdf!!!!! address is {}", addr)
                    }

                }
                VcpuExit::MmioRead(addr, _data) => {
                    println!(
                        "Received an MMIO Read Request for the address {:#x}.",
                        addr,
                    );
                }
                VcpuExit::MmioWrite(addr, _data) => {
                    println!(
                        "Received an MMIO Write Request to the address {:#x}.",
                        addr,
                    );
                }
                VcpuExit::Hlt => {
                    //self.vcpu_fds[0].get_regs(&regs).map_err(Error::IOError)?;
                    //println!("ax is {:x}", regs.rax);


                    println!("get hlt");
                    // The code snippet dirties 1 page when it is loaded in memory
                    /*let dirty_pages_bitmap = self.vm_fd.get_dirty_log(0, self.mem_size).unwrap();
                    let dirty_pages = dirty_pages_bitmap
                        .into_iter()
                        .map(|page| page.count_ones())
                        .fold(0, |dirty_page_count, i| dirty_page_count + i);
                    assert_eq!(dirty_pages, 5);*/
                    break;
                }
                VcpuExit::FailEntry => {
                    println!("get fail entry***********************************");
                    break
                }
                VcpuExit::Exception => {
                    println!("get exception");
                }
                r => panic!("Unexpected exit reason: {:?}", r),
            }
        }

        //let mut vcpu_regs = self.vcpu_fd.get_regs()?;
        Ok(())
    }
}

