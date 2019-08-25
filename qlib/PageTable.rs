#![macro_use]

use alloc::vec::Vec;
use x86_64::structures::paging::{PageTable, PageTableFlags};
use x86_64::PhysAddr;
use x86_64::VirtAddr;
use x86_64::ux::u9;

use super::Common::{Error, Result};
use super::Addr::{Addr, PageOpts};
use alloc::alloc::{Layout, alloc, dealloc};

pub struct PageTables {
    //Root page guest physical address
    pub root: Addr,
}

impl PageTables {
    pub fn New(pagePool: &mut PagePool) -> Result<Self> {
        let root = pagePool.Allocate()?;

        Ok(PageTables{
            //pagePool : pagePool.clone(),
            root: root,
        })
    }

    pub fn Init(root: u64) -> Self {
        return PageTables{
            root: Addr(root),
        }
    }

    pub fn VirtuallToPhy(&self, vaddr: u64) -> Result<u64> {
        let vaddr = VirtAddr::new(vaddr);

        let p4Idx = vaddr.p4_index();
        let p3Idx = vaddr.p3_index();
        let p2Idx = vaddr.p2_index();
        let p1Idx = vaddr.p1_index();
        let pageOffset = vaddr.page_offset();

        let pt: *mut PageTable = self.root.0 as *mut PageTable;

        unsafe {
            let pgdEntry = &(*pt)[p4Idx];
            if pgdEntry.is_unused() {
                return Err(Error::AddressNotMap)
            }

            let pudTbl = pgdEntry.addr().as_u64() as *const PageTable;
            let pudEntry = &(*pudTbl)[p3Idx];
            if pudEntry.is_unused() {
                return Err(Error::AddressNotMap)
            }

            let pmdTbl = pudEntry.addr().as_u64() as *const PageTable;
            let pmdEntry =  &(*pmdTbl)[p2Idx];
            if pmdEntry.is_unused() {
                return Err(Error::AddressNotMap)
            }

            let pteTbl = pmdEntry.addr().as_u64() as *const PageTable;
            let pteEntry =  &(*pteTbl)[p1Idx];
            if pteEntry.is_unused() {
                return Err(Error::AddressNotMap)
            }

            let pageAddr : u64 = pageOffset.into();
            let phyAddr = pteEntry.addr().as_u64() + pageAddr;

            return Ok(phyAddr)
        }

    }

    //return true when there is previous mapping in the range
    pub fn Map(&self, start: Addr, end: Addr, physical: Addr, opts: &PageOpts, pagePool: &mut PagePool) -> Result<bool> {
        //println!("pagetable map start is {:x}, end is {:x}, physical address is {:x}", start.0, end.0, physical.0);

        if !opts.AccessType.Any() {
            return self.Unmap(start, end);
        }

        start.PageAligned()?;
        if end.0 < start.0 {
            return Err(Error::AddressNotInRange);
        }

        if start.0 < super::LOWER_TOP {
            if end.0 <= super::LOWER_TOP {
                return self.mapCanonical(start, end, physical, opts, pagePool)
            } else if end.0 > super::LOWER_TOP && end.0 <= super::UPPER_BOTTOM {
                return self.mapCanonical(start, Addr(super::LOWER_TOP), physical, opts, pagePool)
            } else {
                return self.mapCanonical(start, Addr(super::LOWER_TOP), physical, opts, pagePool)

                //todo: check the physical address
                //self.mapCanonical(UPPER_BOTTOM, end, physical, opts)
            }
        } else if start.0 < super::UPPER_BOTTOM {
            if end.0 > super::UPPER_BOTTOM {
                return self.mapCanonical(Addr(super::UPPER_BOTTOM), end, physical, opts, pagePool)
            }
        } else {
            return self.mapCanonical(start, end, physical, opts, pagePool)
        }

        return Ok(false);
    }

    fn mapCanonical(&self, start: Addr, end: Addr, phyAddr: Addr, _opts: &PageOpts, pagePool: &mut PagePool) -> Result<bool> {
        let mut res = false;

        let mut curAddr = start;

        let pt: *mut PageTable = self.root.0 as *mut PageTable;
        unsafe {
            let mut p4Idx = VirtAddr::new(curAddr.0).p4_index();
            let mut p3Idx = VirtAddr::new(curAddr.0).p3_index();
            let mut p2Idx = VirtAddr::new(curAddr.0).p2_index();
            let mut p1Idx = VirtAddr::new(curAddr.0).p1_index();

            while curAddr.0<end.0 {
                let pgdEntry = &mut (*pt)[p4Idx];
                let pudTbl: *mut PageTable;

                if pgdEntry.is_unused() {
                    pudTbl = pagePool.Allocate()?.0 as *mut PageTable;
                    (*pudTbl).zero();
                    pgdEntry.set_addr(PhysAddr::new(pudTbl as u64),
                                      PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE);


                } else {
                    pudTbl = pgdEntry.addr().as_u64() as *mut PageTable;
                }

                while curAddr.0 < end.0 {
                    let pudEntry = &mut (*pudTbl)[p3Idx];
                    let pmdTbl: *mut PageTable;

                    if pudEntry.is_unused() {
                        pmdTbl = pagePool.Allocate()?.0 as *mut PageTable;
                        (*pmdTbl).zero();
                        pudEntry.set_addr(PhysAddr::new(pmdTbl as u64),
                                          PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE);
                    } else {
                        pmdTbl = pudEntry.addr().as_u64() as *mut PageTable;
                    }

                    while curAddr.0 < end.0 {
                        let pmdEntry =  &mut (*pmdTbl)[p2Idx];
                        let pteTbl: *mut PageTable;

                        if pmdEntry.is_unused() {
                            pteTbl = pagePool.Allocate()?.0 as *mut PageTable;
                            (*pteTbl).zero();
                            pmdEntry.set_addr(PhysAddr::new(pteTbl as u64),
                                              PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE);
                        } else {
                            pteTbl = pmdEntry.addr().as_u64() as *mut PageTable;
                        }

                        while curAddr.0 < end.0 {
                            let pteEntry =  &mut (*pteTbl)[p1Idx];

                            if pteEntry.is_unused() {
                                pteEntry.set_addr(PhysAddr::new(phyAddr.0 + curAddr.0 - start.0), PageTableFlags::PRESENT |PageTableFlags::WRITABLE |PageTableFlags::USER_ACCESSIBLE);
                            } else {
                                res = true;
                            }

                            curAddr = curAddr.AddLen(super::PAGE_SIZE_4K)?;

                            if p1Idx == u9::new(super::ENTRY_COUNT-1) {
                                p1Idx = u9::new(0);
                                break;
                            } else {
                                p1Idx = p1Idx.wrapping_add(u9::new(1));
                            }
                        }

                        if p2Idx == u9::new(super::ENTRY_COUNT-1) {
                            p2Idx = u9::new(0);
                            break;
                        } else {
                            p2Idx = p2Idx.wrapping_add(u9::new(1));
                        }
                    }

                    if p3Idx == u9::new(super::ENTRY_COUNT-1) {
                        p3Idx = u9::new(0);
                        break;
                    } else {
                        p3Idx = p3Idx.wrapping_add(u9::new(1));
                    }
                }

                p4Idx = p4Idx.wrapping_add(u9::new(1));
            }
        }

        return Ok(res);
    }

    pub fn Unmap(&self, _start: Addr, _end: Addr) -> Result<bool> {
        return Ok(true)
    }
}

pub struct GuestPagePool {

}

impl GuestPagePool {
    pub fn new() -> Self {
        return GuestPagePool{}
    }

    pub fn Allocate(&mut self) -> Result<Addr> {
        let layout = Layout::from_size_align(4096, 4096);
        match layout {
            Err(_e) => Err(Error::UnallignedAddress),
            Ok(l) => unsafe {
                let addr = alloc(l);
                Ok(Addr(addr as u64))
            }
        }
    }

    pub fn Free(&mut self, addr: Addr) -> Result<()> {
        let layout = Layout::from_size_align(4096, 4096);
        match layout {
            Err(_e) => Err(Error::UnallignedAddress),
            Ok(l) => unsafe {
                dealloc(addr.0 as *mut u8, l);
                Ok(())
            }
        }
    }
}

pub struct PagePool {
    pub baseAddr : Addr,
    pub next : u32,
    pub pageCount: u32,

    freePool: Vec<u32>,
}

impl PagePool {
    pub fn Init(baseAddr: Addr, pageCount: u32) -> Result<Self> {
        return Ok(PagePool {
            baseAddr: baseAddr,
            next: 0,
            pageCount,
            freePool: Vec::new(),
        });
    }

    pub fn InitWithPara(baseAddr: u64, pageCount: u32, next : u32) -> Self {
        return PagePool {
            baseAddr: Addr(baseAddr),
            next: next,
            pageCount,
            freePool: Vec::new(),
        }
    }

    pub fn Allocate(&mut self) -> Result<Addr> {
        if self.freePool.len() > 0 {
            let idx = self.freePool[self.freePool.len() - 1];
            self.freePool.pop();
            return self.baseAddr.AddLen(idx as u64*super::PAGE_SIZE_4K)
        }

        //println!("next {}, pageCount {}", self.next, self.pageCount);
        if self.next == self.pageCount {
            return Err(Error::NoEnoughMemory)
        }

        let idx = self.next;
        self.next += 1;
        return self.baseAddr.AddLen(idx as u64*super::PAGE_SIZE_4K);
    }

    pub fn Free(&mut self, addr: Addr) -> Result<()> {
        //todo:: check ???
        addr.PageAligned()?;

        let idx = self.baseAddr.PageOffsetIdx(addr)?;

        if idx >= self.pageCount {
            return Err(Error::AddressNotInRange);
        }

        self.freePool.push(idx);
        return Ok(());
    }

    pub fn GetPageIdx(&self, addr: Addr) -> Result<u32> {
        self.baseAddr.PageOffsetIdx(addr)
    }

    pub fn GetPageAddr(&self, idx: u32) -> Result<Addr> {
        if idx >= self.pageCount {
            return Err(Error::AddressNotInRange);
        }

        return Ok(self.baseAddr.AddPages(idx));
    }
}