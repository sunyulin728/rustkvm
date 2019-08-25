#![allow(non_snake_case)]
#![allow(dead_code)]

use std::collections::HashMap;

use super::Addr::{Addr};
use super::qlib::Common::Error;
use super::qlib::Common::Result;

pub const PAGE_SIZE_4K : u64 = 0x1000;
pub const PAGE_SIZE_2M : u64 = (2*ONE_MB);
pub const PAGE_MASK : u64 = 0xffff;
pub const ONE_MB: u64 = 0x100_000;
pub const ONE_GB: u64 = 0x40_000_000;
pub const STACK_SIZE : u64 = 6 * ONE_MB;
pub const STACK_GUARDPAGE_SIZE : u64 = 2 * ONE_MB;
pub const BLOCK_SIZE:  u64 = 64 * ONE_GB;
pub const PHY_UPPER_ADDR: u64 = 7 * BLOCK_SIZE;
pub const KERNEL_ADDR_SIZE: u64 = 128 * BLOCK_SIZE;
pub const PHY_MEM_SPACE : u64 = 8*BLOCK_SIZE;
pub const LOWER_TOP : u64 = 0x00007fffffffffff;
pub const UPPER_BOTTOM : u64 = 0xffff800000000000;
pub const ENTRY_COUNT: u16 = 512 as u16;

trait AddrRange {
    fn Start() -> u64;
    fn End() -> u64;
}

trait PhyRegionTrait {
    fn HostStartAddr(&self) -> Addr;
    fn PhyStartAddr(&self) -> Addr;
    fn Len(&self) -> u64;
}


pub struct Range {
    start : Addr,
    end : Addr,
}

pub struct FileInfo {
    offset: Addr,
    hostFileName: String,
}

pub struct PhyRegion {
    fileInfo : Option<FileInfo>,

    hostBaseAddr: Addr,
    mr: Box<MappedRegion>,
}

impl PhyRegion {
    pub fn InitAnan(hostBaseAddr: Addr, hostAddrLimit:Addr, len: u64, hugePage: bool) -> Result<PhyRegion> {
        let mut option = &mut MapOption::New();
        option = option.Offset(hostBaseAddr.0).Len(len).MapAnan().MapPrivate().ProtoRead().ProtoWrite().ProtoExec();
        if hugePage {
            option = option.MapHugeTLB();
        }

        let mr = Box::new(option.Map()?);
        //let mr = Box::new(MappedRegion::Init(hostBaseAddr, len, hugePage, libc::PROT_READ |  libc::PROT_WRITE |  libc::PROT_EXEC)?);
        if mr.End()?.0 >= hostAddrLimit.0 {
            return Err(Error::AddressNotInRange);
        }

        return Ok(PhyRegion{
            fileInfo: None,
            hostBaseAddr: hostBaseAddr,
            mr: mr,
        })
    }

    fn HostStartAddr(&self) -> Addr  {
        return self.mr.Start();
    }

    fn PhyStartAddr(&self) -> Addr {
        return self.mr.Start().Offset(self.hostBaseAddr).unwrap();
    }

    fn Len(&self) -> u64 {
        return self.mr.Len()
    }

    fn IsAnan(&self) -> bool {
        match self.fileInfo {
            None => true,
            _ => false
        }
    }
}

pub struct PhyAddrMgr {
    hostBaseAddr: Addr,
    hostAddrLimit: Addr,
    regions: HashMap<u64, Box<PhyRegion>>,
}

impl PhyAddrMgr {
    pub fn Init(hostBaseAddr: Addr, len: u64) -> Result<Self> {
        return Ok(PhyAddrMgr {
            hostBaseAddr: hostBaseAddr,
            hostAddrLimit: hostBaseAddr.AddLen(len)?,
            regions: HashMap::new(),
        })
    }

    //rely on host OS to manage the range
    //return guest phyical start address
    pub fn AllocAnan(&mut self, len: u64, hugePage: bool) -> Result<Addr> {
        let region = Box::new(PhyRegion::InitAnan(self.hostBaseAddr, self.hostAddrLimit, len, hugePage)?);
        let ret = region.PhyStartAddr();
        self.regions.insert(ret.0, region);
        Ok(ret)
    }

    pub fn PhyToHostAddr(&mut self, phyStartAddr: Addr) -> Result<Addr> {
        if let Some(region) = self.regions.get(&phyStartAddr.0) {
            return Ok (region.HostStartAddr());
        } else {
            return Err(Error::UnmatchRegion)
        }
    }

    //start is the physical address
    pub fn Free(&mut self, start: u64, len: u64) -> Result<()> {
        if let Some(region) = self.regions.get(&start) {
            if region.Len() != len {
                return Err(Error::UnmatchRegion)
            }
        } else {
            return Err(Error::UnmatchRegion)
        }

        self.regions.remove(&start);
        Ok(())
    }
}

#[derive(Debug)]
pub struct MapOption {
    offset: u64,
    len: u64,
    flags: libc::c_int,
    proto: libc::c_int,
    fd: libc::c_int,
    fileOffset: libc::off_t,
}

impl MapOption {
    pub fn New() -> Self {
        return MapOption {
            offset: 0,
            len: 0,
            flags: 0,
            proto: libc::PROT_NONE,
            fd: -1,
            fileOffset: 0
        }
    }

    pub fn FileId(&mut self, id: i32) -> &mut Self {
        self.fd = id as libc::c_int;
        self
    }

    pub fn Offset(&mut self, offset: u64) -> &mut Self {
        self.offset = offset;
        self
    }

    pub fn Len(&mut self, len: u64) -> &mut Self {
        self.len = len;
        self
    }

    pub fn FileOffset(&mut self, offset: u64) -> &mut Self {
        self.fileOffset = offset as libc::off_t;
        self
    }

    pub fn ProtoExec(&mut self) -> &mut Self {
        self.proto |= libc::PROT_EXEC;
        self
    }

    pub fn ProtoRead(&mut self) -> &mut Self {
        self.proto |= libc::PROT_READ;
        self
    }

    pub fn ProtoWrite(&mut self) -> &mut Self {
        self.proto |= libc::PROT_WRITE;
        self
    }

    pub fn MapShare(&mut self) -> &mut Self {
        self.flags |= libc::MAP_SHARED;
        self
    }

    pub fn MapShareValidate(&mut self) -> &mut Self {
        self.flags |= libc::MAP_SHARED_VALIDATE;
        self
    }

    pub fn MapPrivate(&mut self) -> &mut Self {
        self.flags |= libc::MAP_PRIVATE;
        self
    }

    pub fn MapAnan(&mut self) -> &mut Self {
        self.flags |= libc::MAP_ANONYMOUS;
        self
    }

    pub fn MapFixed(&mut self) -> &mut Self {
        self.flags |= libc::MAP_FIXED;
        self
    }

    pub fn MapFixedNoReplace(&mut self) -> &mut Self {
        self.flags |= libc::MAP_FIXED_NOREPLACE;
        self
    }

    pub fn MapHugeTLB(&mut self) -> &mut Self {
        self.flags |= libc::MAP_HUGETLB;
        self
    }

    pub fn MapLocked(&mut self) -> &mut Self {
        self.flags |= libc::MAP_LOCKED;
        self
    }

    pub fn MapNonBlock(&mut self) -> &mut Self {
        self.flags |= libc::MAP_NONBLOCK;
        self
    }

    pub fn Map(&self) -> Result<MappedRegion> {
        MappedRegion::New(
            self.offset as *mut libc::c_void,
            self.len as libc::size_t,
            self.proto as libc::c_int,
            self.flags as libc::c_int,
            self.fd as libc::c_int,
            self.fileOffset as libc::off_t
        )
    }
}

#[derive(Debug)]
pub struct MappedRegion {
    pub sz: u64,
    pub ptr: u64,
}

impl MappedRegion {
    pub fn New(addr: *mut libc::c_void, len: libc::size_t, prot: libc::c_int, flags: libc::c_int, fd: libc::c_int, offset: libc::off_t) ->  Result<Self> {
        unsafe {
            let addr = libc::mmap(addr,
                                  len,
                                  prot,
                                  flags,
                                  fd,
                                  offset);

            if (addr as i64) < 0 {
                Err(Error::MMampError)
            } else {
                /*println!("mmap address is {:x} len is {:x} offset is {:x}, fd is {}, proto is {:x}, flags is {:x}",
                         addr as u64, len as u64, offset as u64, fd as u32, prot as u64, flags as u64);

                if fd != -1 {
                    println!("mmap value is:");

                    for i in 0..len {
                        print!("{:x} ", *((addr as u64 +i as u64) as *const u8))
                    }

                    println!("");
                }*/

                Ok( MappedRegion {
                    ptr: addr as u64,
                    sz: len as u64,
                })
            }
        }
    }

    pub fn as_ptr(&self) -> *mut u8 {
        return self.ptr as *mut u8;
    }
    pub fn Start(&self) -> Addr {
        return Addr(self.ptr as u64)
    }

    pub fn End(&self) -> Result<Addr>  {
        return self.Start().AddLen(self.sz)
    }

    pub fn Len(&self) -> u64 {
        return self.sz
    }
}

impl Drop for MappedRegion {
    fn drop(&mut self) {
        unsafe {
            println!("unmap ptr is {:x}, len is {:x}", self.ptr as u64, self.sz);

            if libc::munmap(self.ptr as *mut libc::c_void, self.sz as usize) != 0 {
                panic!("munmap: {}", std::io::Error::last_os_error());
            }
        }
    }
}
