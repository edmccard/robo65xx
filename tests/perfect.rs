use std::{
    ffi::{c_int, c_void},
    fmt::Display,
};

// http://www.visual6502.org/JSSim/expert.html?graphics=f&loglevel=2&steps=50&a=0010&d=58e60f38690260&a=fffe&d=2000&a=0020&d=e840&r=0010&irq0=20&irq1=26&logmore=irq,INTG,ir

unsafe extern "C" {
    fn initAndResetChip() -> *mut c_void;
    fn destroyChip(state: *mut c_void);
    fn readAddressBus(state: *mut c_void) -> u16;
    fn readDataBus(state: *mut c_void) -> u8;
    fn writeDataBus(state: *mut c_void, d: u8);
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
    fn step(state: *mut c_void) -> c_int;
}

struct P6502(*mut c_void);

impl P6502 {
    fn new(mem: &mut [u8]) -> Self {
        let mut cpu = unsafe {
            let state = initAndResetChip();
            Self(state)
        };
        for i in 0..19 {
            cpu.step(mem);
        }
        cpu
    }

    fn step(&mut self, mem: &mut [u8]) {
        unsafe {
            if step(self.0) != 0 {
                let a = readAddressBus(self.0);
                if self.write() {
                    mem[a as usize] = readDataBus(self.0);
                } else {
                    writeDataBus(self.0, mem[a as usize]);
                }
            }
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
    let mut pmem = [0u8; 65536];
    let pcpu = P6502::new(&mut pmem);
    println!("P {}", pcpu);

    let mut rmem = [0u8; 65536];
    let mut ctrl = Control::init();
    let mut bus = Bus::new();
    let mut cpu = CPU::init(&bus, &mut ctrl);
    for _ in 0..8 {
        cpu.tick(&mut bus, &mut ctrl);
        if bus.rw() {
            bus.data = rmem[bus.addr as usize];
        } else {
            rmem[bus.addr as usize] = bus.data;
        }
    }
    println!("R {} {} {}", &ctrl, &bus, &cpu);
}
