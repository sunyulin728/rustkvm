use super::qlib::Common::{Result};
use super::qlib::PageTable::{PagePool, PageTables};
use super::qlib::Addr::{Addr, PageOpts};

pub struct VMSpace {
    pub pagePool: Option<PagePool>,
    pub pageTables : Option<PageTables>,
}

impl VMSpace {
    pub fn Map(&mut self, start: Addr, end: Addr, physical: Addr, opts: &PageOpts) -> Result<bool> {
        return self.pageTables.as_mut().unwrap().Map(start, end, physical, opts, self.pagePool.as_mut().unwrap());
    }

}

impl Default for VMSpace {
    fn default() -> Self {
        return VMSpace {
            pagePool: None,
            pageTables: None,
        }
    }
}