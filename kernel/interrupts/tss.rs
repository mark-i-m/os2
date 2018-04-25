//! A module for 64-bit TSS
//!
//! [See Intel x86_64 Manual, Vol 3A, Chapter 7.7]
//! (https://software.intel.com/sites/default/files/managed/7c/f1/253668-sdm-vol-3a.pdf)

use core::mem;

use machine::ltr;

static mut TSS64: TSS = TSS::new();

extern "C" {
    static mut tssDescriptor: TSSDescriptor;
    static tssDS: usize; // TODO: check all of the types in this file to make sure they are the right width

    #[allow(dead_code)]
    static kernelDataSeg: u16;
}

#[repr(C, packed)]
struct TSSDescriptor {
    f0: u32,
    f1: u32,
    f2: u32,
    f3: u32,
}

#[allow(dead_code)]
#[repr(C, packed)]
struct TSS {
    entries: [u32; 25],
}

impl TSSDescriptor {
    fn set(&mut self, base: &'static TSS) {
        let limit = mem::size_of::<TSS>() as u32;
        let base = base as *const TSS as u64;
        let base_lower = (base & 0xFFFF_FFFF) as u32;
        let base_upper = (base >> 32) as u32;

        // clear
        self.f0 = 0;
        self.f1 = 0;
        self.f2 = 0;
        self.f3 = 0;

        // f0 [bbbbbbbbbbbbbbbbllllllllllllllll]
        // f1 [bbbbbbbb    llllp   ttttbbbbbbbb]
        // f2 [bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb]
        // f3 [00000000000000000000000000000000]

        // set base
        self.f0 = (self.f0 & 0x0000_FFFF) | ((base_lower << 16) & 0xFFFF_0000);
        self.f1 = (self.f1 & 0xFFFF_FF00) | ((base_lower >> 16) & 0x0000_00FF);
        self.f1 = (self.f1 & 0x00FF_FFFF) | (base_lower & 0xFF00_0000);
        self.f2 = base_upper;

        // set limit
        self.f0 = (self.f0 & 0xFFFF_0000) | (limit & 0x0000_FFFF);
        self.f1 = (self.f1 & 0xFFF0_FFFF) | (limit & 0x000F_0000);

        // set lots of flags here (some of them are being set to 0):

        // set "accessed" to indicate TSS, not LDT
        self.f1 |= 1 << 8;

        // set "executable"
        self.f1 |= 1 << 11;

        // 64-bit TSS
        self.f1 |= 1 << 21;

        // set present
        self.f1 |= 1 << 15;
    }
}

impl TSS {
    const fn new() -> TSS {
        TSS { entries: [0; 25] }
    }

    fn rsp0(&mut self, v: usize) {
        let lower: u32 = (v & 0xFFFF_FFFF) as u32;
        let upper: u32 = (v >> 32) as u32;

        // set RSP0
        self.entries[1] = lower;
        self.entries[2] = upper;
    }
}

pub fn init() {
    unsafe {
        tssDescriptor.set(&TSS64);
        ltr(tssDS);
    }
}

#[allow(dead_code)]
pub fn rsp0(v: usize) {
    unsafe {
        TSS64.rsp0(v);
    }
}
