// TSS module

use core::mem;

use machine::ltr;

static mut TSS_: TSS = TSS::new();

extern "C" {
    static mut tssDescriptor: TSSDescriptor;
    static tssDS: usize;
    static kernelDataSeg: u16;
}

#[repr(C,packed)]
struct TSSDescriptor {
    f0: usize,
    f1: usize,
}

#[allow(dead_code)]
#[repr(C,packed)]
struct TSS {
    prev: usize,
    esp0: usize,
    ss0: usize,
    esp1: usize,
    ss1: usize,
    esp2: usize,
    ss2: usize,
    unused: [usize; 19],
}

impl TSSDescriptor {
    fn set(&mut self, base: &'static TSS) {
        let limit = mem::size_of::<TSS>();
        let base_usize = base as *const TSS as usize;

        // clear
        self.f0 = 0;
        self.f1 = 0;

        // f0 [bbbbbbbbbbbbbbbbllllllllllllllll]
        // f1 [bbbbbbbb    llllp   ttttbbbbbbbb]

        // set base
        self.f0 = (self.f0 & 0x0000_FFFF) | ((base_usize << 16) & 0xFFFF_0000);
        self.f1 = (self.f1 & 0xFFFF_FF00) | ((base_usize >> 16) & 0x0000_00FF);
        self.f1 = (self.f1 & 0x00FF_FFFF) | (base_usize & 0xFF00_0000);

        // set limit
        self.f0 = (self.f0 & 0xFFFF_0000) | (limit & 0x0000_FFFF);
        self.f1 = (self.f1 & 0xFFF0_FFFF) | (limit & 0x000F_0000);

        // set lots of flags here (some of them are being set to 0):

        // set "accessed" to indicate TSS, not LDT
        self.f1 |= 1 << 8;

        // set "executable"
        self.f1 |= 1 << 11;

        // 32-bit TSS
        self.f1 |= 1 << 22;

        // set present
        self.f1 |= 1 << 15;
    }
}

impl TSS {
    const fn new() -> TSS {
        TSS {
            prev: 0,
            esp0: 0,
            ss0: 0,
            esp1: 0,
            ss1: 0,
            esp2: 0,
            ss2: 0,
            unused: [0; 19],
        }
    }

    fn esp0(&mut self, v: usize) {
        self.esp0 = v;
    }
}

pub fn init() {
    unsafe {
        TSS_.ss0 = kernelDataSeg as usize;
        tssDescriptor.set(&TSS_);
        ltr(tssDS);
    }
}

pub fn esp0(v: usize) {
    unsafe {
        TSS_.esp0(v);
    }
}
