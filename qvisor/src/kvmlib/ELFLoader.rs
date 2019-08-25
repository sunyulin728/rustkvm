//use super::MemMgr::MemSpaceMgr;

use std::slice;

use super::VMS;

use super::Addr::Addr;
use super::Addr::PageOpts;

//use xmas_elf::dynamic::Tag;
//use xmas_elf::header;
use xmas_elf::program::ProgramHeader::{Ph64};
use xmas_elf::program::Type;
//use xmas_elf::program::{ProgramIter, SegmentData, Type};
//use xmas_elf::sections::SectionData;
use xmas_elf::*;

pub use xmas_elf::program::{Flags, ProgramHeader, ProgramHeader64};
pub use xmas_elf::sections::Rela;
pub use xmas_elf::symbol_table::{Entry, Entry64};
pub use xmas_elf::{P32, P64};
pub use xmas_elf::header::HeaderPt2;

use memmap::Mmap;
use super::qlib::Common::Error;
use super::qlib::Common::Result;

use std::fs::File;

use super::MemMgr::{MappedRegion, MapOption};

pub struct KernelELF {
    mmap: Mmap,
    startAddr: Addr,
    endAddr: Addr,
    entry: u64,
    mr: Option<MappedRegion>,
}

impl KernelELF {
    pub fn Init(fileName: &String) -> Result<Self> {
        let f = File::open(fileName).map_err(|e| Error::IOError(format!("io::error is {:?}", e)))?;
        let mmap = unsafe { Mmap::map(&f).map_err(|e| Error::IOError(format!("io::error is {:?}", e)))? };
        let elfFile = ElfFile::new(&mmap).map_err(|e| Error::IOError(format!("io::error is {:?}", e)))?;
        let mut startAddr: Addr = Addr(0xfffff_fffff_fffff);
        let mut endAddr: Addr = Addr(0);

        let entry = match &elfFile.header.pt2 {
            HeaderPt2::Header64(pt2) => pt2.entry_point,
            _ => return Err(Error::WrongELFFormat),
        };

        for p in elfFile.program_iter() {
            //todo : add more check
            if let Ph64(header) = p  {
                if header.get_type().map_err(Error::ELFLoadError)? == Type::Load {
                    let startMem = Addr(header.virtual_addr).RoundDown()?;
                    let endMem = Addr(header.virtual_addr).AddLen(header.file_size)?.RoundUp()?;

                    if startMem.0 < startAddr.0 {
                        startAddr = startMem;
                    }

                    if endAddr.0 < endMem.0 {
                        endAddr = endMem;
                    }
                }
            }
        }

        return Ok(KernelELF {
            mmap,
            startAddr,
            endAddr,
            entry,
            mr : None,
        })
    }

    pub fn StartAddr(&self) -> Addr {
        return self.startAddr;
    }

    pub fn EndAddr(&self) -> Addr {
        return self.endAddr;
    }

    pub fn LoadKernel(&mut self) -> Result<u64> {
        let mut option = &mut MapOption::New();
        option = option.Offset(self.startAddr.0).Len(self.endAddr.0 - self.startAddr.0).MapAnan().MapPrivate().ProtoRead().ProtoWrite().ProtoExec();

        let mr = option.Map()?;
        //let mr = MappedRegion::Init(self.startAddr, self.endAddr.0 - self.startAddr.0, false, libc::PROT_READ |  libc::PROT_WRITE |  libc::PROT_EXEC)?;
        let hostAddr = Addr(mr.ptr as u64);
        if hostAddr.0 != self.startAddr.0 {
            return Err(Error::AddressDoesMatch)
        }

        println!("loadKernel: get address is {:x}", mr.ptr as u64);

        let elfFile = ElfFile::new(&self.mmap).map_err(Error::ELFLoadError)?;
        for p in elfFile.program_iter() {
            //todo : add more check
            if let Ph64(header) = p  {
                if header.get_type().map_err(Error::ELFLoadError)? == Type::Load {
                    let startMem = Addr(header.virtual_addr).RoundDown()?;
                    let pageOffset = Addr(header.virtual_addr).0 - Addr(header.virtual_addr).RoundDown()?.0;
                    let endMem = Addr(header.virtual_addr).AddLen(header.file_size)?.RoundUp()?;

                    let target = unsafe { slice::from_raw_parts_mut((startMem.0+pageOffset) as *mut u8, header.file_size as usize) };
                    let source = &self.mmap[header.offset as usize..(header.offset+header.file_size) as usize];

                    target.clone_from_slice(source);

                    VMS.lock().Map(startMem, endMem, startMem, &PageOpts::Default())?;
                }
            }
        }

        self.mr = Some(mr);

        return Ok(self.entry)
    }
}

//return entry
/*pub fn LoadKernel1(fileName: &String, pt: &mut PageTables, pM: &mut PhyAddrMgr) -> Result<(u64)> {
    let f = File::open(fileName).map_err(|e| Error::IOError(format!("io::error is {:?}", e)))?;
    let mmap = unsafe { Mmap::map(&f).map_err(|e| Error::IOError(format!("io::error is {:?}", e)))? };
    let elfFile = ElfFile::new(&mmap).map_err(Error::ELFLoadError)?;

    let entry = match &elfFile.header.pt2 {
        HeaderPt2::Header64(pt2) => pt2.entry_point,
        _ => return Err(Error::WrongELFFormat),
    };

    let mut size : u64 = 0;

    for p in elfFile.program_iter() {
        //todo : add more check
        if let Ph64(header) = p  {
            if header.get_type().map_err(Error::ELFLoadError)? == Type::Load {
                let startMem = Addr(header.virtual_addr).RoundDown()?;
                let endMem = Addr(header.virtual_addr).AddLen(header.file_size)?.RoundUp()?;

                size += endMem.0 - startMem.0;
            }
        }
    }

    let phyAddr = pM.AllocAnan(size, false)?;
    let hostAddr = pM.PhyToHostAddr(phyAddr)?;

    let mut offset : u64 = 0;
    for p in elfFile.program_iter() {
        //todo : add more check
        if let Ph64(header) = p  {
            if header.get_type().map_err(Error::ELFLoadError)? == Type::Load {
                let startMem = Addr(header.virtual_addr).RoundDown()?;
                let pageOffset = Addr(header.virtual_addr).0 - Addr(header.virtual_addr).RoundDown()?.0;
                let endMem = Addr(header.virtual_addr).AddLen(header.file_size)?.RoundUp()?;
                let len = endMem.0 - startMem.0;

                let target = unsafe { slice::from_raw_parts_mut((hostAddr.0+pageOffset) as *mut u8, header.file_size as usize) };
                let source = &mmap[header.offset as usize..(header.offset+header.file_size) as usize];

                target.clone_from_slice(source);

                pt.Map(startMem, endMem, phyAddr.AddLen(offset)?, &PageOpts::Default())?;

                offset += len;
            }
        }
    }

    return Ok(entry)
}


#[derive(Debug)]
pub struct ProgSec {
    pub virutalAddr: Addr,
    pub mmap : Mmap,
}

#[derive(Debug)]
pub struct Loader1 {
    pub entry: u64,
    pub progSecs : Vec<ProgSec>,
}

impl Loader1 {
    pub fn Init(fileName: &String) -> Result<Self> {
        let f = File::open(fileName).map_err(|e| Error::IOError(format!("io::error is {:?}", e)))?;
        let mmap = unsafe { Mmap::map(&f).map_err(|e| Error::IOError(format!("io::error is {:?}", e)))? };
        let elfFile = ElfFile::new(&mmap).map_err(Error::ELFLoadError)?;

        let entry = match &elfFile.header.pt2 {
            HeaderPt2::Header64(pt2) => pt2.entry_point,
            _ => return Err(Error::WrongELFFormat),
        };

        let mut res = Loader1 {
            entry : entry,
            progSecs : Vec::new(),
        };

        for p in elfFile.program_iter() {
            //todo : add more check
            if let Ph64(header) = p  {
                if header.get_type().map_err(Error::ELFLoadError)? == Type::Load {
                    let startMem = Addr(header.virtual_addr).RoundDown()?;
                    let pageOffset = Addr(header.virtual_addr).0 - Addr(header.virtual_addr).RoundDown()?.0;
                    let endMem = Addr(header.virtual_addr).AddLen(header.file_size)?.RoundUp()?;

                    //println!("{}, {}, {}", startMem.0, Addr(header.virtual_addr).AddLen(header.file_size)?.0, endMem.0 );

                    let mut targetMap: MmapMut = MmapOptions::new().len(endMem.0 as usize-startMem.0 as usize).map_anon().
                        map_err(|e| Error::CreateMMap(format!("io::error is {:?}", e)))?;
                    let offset = header.offset;

                    for i in 0..pageOffset as usize {
                        targetMap[i] = 0
                    }

                    for i in (pageOffset+header.file_size) as usize..targetMap.len() {
                        targetMap[i] = 0
                    }

                    //println!("{}, {}, {}, {}", 0, pageOffset, pageOffset+header.file_size, targetMap.len());

                    targetMap[pageOffset as usize..(pageOffset+header.file_size) as usize].clone_from_slice(&mmap[offset as usize..(offset+header.file_size) as usize]);
                    res.progSecs.push(ProgSec{virutalAddr:startMem, mmap:targetMap.make_read_only().
                        map_err(|e| Error::CreateMMap(format!("io::error is {:?}", e)))?});
                }
            }
        }

        return Ok(res);
    }
}

pub fn elftest1() {
    let f = File::open("/home/brad/rust/quark/qkernel/build/kernel-x86_64.bin").expect("fail");
    // let mut buffer = Vec::new();
    // read the whole file
    //f.read_to_end(&mut buffer).expect("fail");

    let mmap = unsafe { Mmap::map(&f).expect("mmap fail")  };
    let buffer = &mmap;

    let elf_file = ElfFile::new(&buffer).expect("fail");
    println!("entry is {}", elf_file.header.pt2.entry_point());

    for p in elf_file.program_iter() {
        if let Ph64(header) = p {
            println!("header type is {}", header);
        }
    }
} */