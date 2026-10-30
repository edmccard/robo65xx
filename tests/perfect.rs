use std::{
    ffi::{c_int, c_void},
    fmt::Display,
};

unsafe extern "C" {
    fn initAndResetChip() -> *mut c_void;
    fn destroyChip(state: *mut c_void);
    fn readAddressBus(state: *mut c_void) -> u16;
    fn readDataBus(state: *mut c_void) -> u8;
    fn readRW(state: *mut c_void) -> c_int;
    fn readA(state: *mut c_void) -> u8;
    fn readX(state: *mut c_void) -> u8;
    fn readY(state: *mut c_void) -> u8;
    fn readP(state: *mut c_void) -> u8;
    fn readIR(state: *mut c_void) -> u8;
    fn readSP(state: *mut c_void) -> u8;
    fn readPC(state: *mut c_void) -> u16;
    fn isNodeHigh(state: *mut c_void, nn: u16) -> c_int;
    fn setNode(state: *mut c_void, nn: u16, s: c_int);
    fn step(state: *mut c_void);
}

struct P6502(*mut c_void);

impl P6502 {
    fn new() -> Self {
        unsafe {
            let state = initAndResetChip();
            for _ in 0..9 {
                step(state);
                step(state);
            }
            Self(state)
        }
    }

    fn addr(&self) -> u16 {
        unsafe { readAddressBus(self.0) }
    }

    fn data(&self) -> u8 {
        unsafe { readDataBus(self.0) }
    }

    fn write(&self) -> bool {
        unsafe { readRW(self.0) == 0 }
    }

    fn intg(&self) -> bool {
        unsafe { isNodeHigh(self.0, 1350) != 0 }
    }

    fn a(&self) -> u8 {
        unsafe { readA(self.0) }
    }

    fn x(&self) -> u8 {
        unsafe { readX(self.0) }
    }

    fn y(&self) -> u8 {
        unsafe { readY(self.0) }
    }

    fn flags(&self) -> u8 {
        unsafe { readP(self.0) }
    }

    fn op(&self) -> u8 {
        unsafe { readIR(self.0) }
    }

    fn sp(&self) -> u8 {
        unsafe { readSP(self.0) }
    }

    fn pc(&self) -> u16 {
        unsafe { readPC(self.0) }
    }

    fn sync(&self) -> bool {
        unsafe { isNodeHigh(self.0, 539) != 0 }
    }

    fn res(&self) -> bool {
        unsafe { isNodeHigh(self.0, 159) == 0 }
    }

    fn nmi(&self) -> bool {
        unsafe { isNodeHigh(self.0, 1297) == 0 }
    }

    fn set_nmi(&mut self, s: bool) {
        unsafe {
            setNode(self.0, 1297, !s as c_int);
        }
    }

    fn irq(&self) -> bool {
        unsafe { isNodeHigh(self.0, 103) == 0 }
    }

    fn set_irq(&mut self, s: bool) {
        unsafe {
            setNode(self.0, 103, !s as c_int);
        }
    }
}

impl Display for P6502 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}{}{} {} {:04x} {:02x} {} : {:04x} {:02x} {:02x} {:02x} {:02x} {:02x} {}",
            if self.sync() { "S" } else { "s" },
            if self.res() { "R" } else { "r" },
            if self.nmi() { "N" } else { "n" },
            if self.irq() { "I" } else { "i" },
            if self.write() { "W" } else { "R" },
            self.addr(),
            self.data(),
            if self.intg() { "!" } else { "." },
            self.pc(),
            self.op(),
            self.a(),
            self.x(),
            self.y(),
            self.sp(),
            Flags(self.flags())
        )
    }
}

impl Drop for P6502 {
    fn drop(&mut self) {
        unsafe {
            destroyChip(self.0);
        }
    }
}

use robo65xx::*;

#[test]
fn test_1() {
    let _foo = P6502::new();
    println!("{}", _foo);

    let mut ctrl = Control::init();
    let mut bus = Bus::new();
    let mut cpu = CPU::init(&bus, &mut ctrl);
    for _ in 0..7 {
        cpu.tick(&mut bus, &mut ctrl);
        // do mem here
    }
    println!("{} {} {}", &ctrl, &bus, &cpu);
}
