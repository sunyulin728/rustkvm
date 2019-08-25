use super::Common::Result;
use super::Common::Error;


pub const PAGE_SIZE : u64 = 0x1000;
pub const ONE_TB:  u64 = 0x1_000_000_000; //0x10_000_000_000;
pub const KERNEL_BASE_ADDR: u64 = 7 * ONE_TB;
pub const KERNEL_ADDR_SIZE: u64 = 128 * ONE_TB;
pub const PHY_MEM_SPACE : u64 = 8*ONE_TB;
pub const PAGE_MASK : u64 = PAGE_SIZE-1;

pub struct AccessType {
    pub Read : bool,
    pub Write: bool,
    pub Exec : bool,
}

impl AccessType {
    pub fn Any(&self) -> bool {
        return self.Read || self.Write || self.Exec
    }
}

pub struct PageOpts {
    pub AccessType : AccessType,
    pub Global: bool,
    pub User: bool,
}

impl PageOpts {
    pub fn Default() -> Self {
        return PageOpts {
            AccessType : AccessType{
                Read: true,
                Write: true,
                Exec: true,
            },
            Global: true,
            User: true,
        }
    }
}


#[derive(Debug, Copy, Clone)]
pub struct Addr (pub u64);

impl Addr {
    pub fn AddLen(&self, len: u64) -> Result<Addr> {
        let end = self.0 + len;
        if end < self.0 {
            Err(Error::Overflow)
        } else {
            Ok(Addr(end))
        }
    }

    pub fn RoundDown(&self) -> Result<Addr> {
        return Ok(Addr(self.0 & !(PAGE_SIZE-1)))
    }

    pub fn RoundUp(&self) -> Result<Addr> {
        let addr = self.0 + PAGE_SIZE -1;
        if addr < self.0 {
            Err(Error::Overflow)
        } else {
            Addr(addr).RoundDown()
        }
    }

    pub fn PageOffset(&self) -> u64 {
        self.0 & (PAGE_SIZE-1)
    }

    pub fn IsPageAligned(&self) -> bool {
        self.PageOffset() == 0
    }

    pub fn PageAligned(&self) -> Result<()> {
        if !self.IsPageAligned() {
            return Err(Error::UnallignedAddress)
        }

        Ok(())
    }

    pub fn AddPages(&self, pageCount: u32) -> Addr {
        return Addr(self.0+pageCount as u64 * PAGE_SIZE)
    }

    pub fn PageOffsetIdx(&self, addr: Addr) -> Result<u32> {
        let addr = addr.RoundDown()?;

        if addr.0 < self.0 {
            return Err(Error::AddressNotInRange);
        }

        return Ok(((addr.0-self.0)/PAGE_SIZE as u64) as u32)
    }

    pub fn Offset(&self, startAddr: Addr) -> Result<Addr> {
        if self.0 < startAddr.0 {
            return Err(Error::AddressNotInRange)
        }

        return Ok(Addr(self.0 - startAddr.0))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct AddrRange {
    pub Start : Addr,
    pub End: Addr,
}

impl AddrRange {
    pub fn IsPageAligned(&self) -> bool {
        self.Start.IsPageAligned() && self.End.IsPageAligned()
    }
}

pub struct RangeMgr {
    pub start : Addr,
    pub end:    Addr,
    offset: Addr,
}

impl RangeMgr {
    pub fn Init(start : Addr, len : u64) -> Result<Self> {
        return Ok(RangeMgr{start, end: start.AddLen(len)?, offset:start})
    }

    pub fn Allocate(&mut self, len : u64) -> Result<Addr> {
        if len & PAGE_MASK != len {
            return Err(Error::UnallignedSize);
        }

        let res = self.offset;
        if self.offset.0 + len > self.end.0 {
            Err(Error::NoEnoughSpace)
        } else {
            self.offset = self.offset.AddLen(len)?;
            Ok(res)
        }
    }

    //take a predefine the range, if the range have been occupied, return error
    pub fn OccupyRange(&self, _offset: u64, _len:u64) -> Result<()> {
        //
        return Err(Error::RangeUnavailable)
    }

    pub fn Free(&mut self, _start: u64, _len : u64) -> Result<()> {
        //todo: fix
        Ok(())
    }
}