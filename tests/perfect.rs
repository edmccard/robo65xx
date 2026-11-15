use std::{
    ffi::{c_int, c_void},
    fmt::Display,
};

use robo65xx::*;

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
    fn new() -> Self {
        unsafe { Self(initAndResetChip()) }
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

    fn rdy(&self) -> bool {
        unsafe { isNodeHigh(self.0, 89) == 0 }
    }

    fn active_states(&self) -> [bool; 7] {
        unsafe {
            [
                isNodeHigh(self.0, 1536) == 0, // clock1
                isNodeHigh(self.0, 156) == 0,  // clock2
                isNodeHigh(self.0, 971) == 0,  // t2
                isNodeHigh(self.0, 1567) == 0, // t3
                isNodeHigh(self.0, 690) == 0,  // t4
                isNodeHigh(self.0, 909) == 0,  // t5
                isNodeHigh(self.0, 1481) != 0, // VEC1
            ]
        }
    }
}

impl Drop for P6502 {
    fn drop(&mut self) {
        unsafe {
            destroyChip(self.0);
        }
    }
}

impl Display for P6502 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}{}{}{} {} {:04x} {:02x} {} {:04x} {:02x} {:02x} {:02x} {:02x} {:02x} {} {}",
            if self.sync() { "S" } else { "s" },
            if self.res() { "R" } else { "r" },
            if self.nmi() { "N" } else { "n" },
            if self.irq() { "I" } else { "i" },
            if self.rdy() { "*" } else { "," },
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
            Flags(self.flags()),
            ["T0", "T1", "T2", "T3", "T4", "T5", "T6"]
                .iter()
                .zip(self.active_states())
                .filter(|&(_, active)| active)
                .map(|(&s, _)| s)
                .collect::<Vec<_>>()
                .join("+")
        )
    }
}

struct TestCase {
    data: Vec<(usize, Vec<u8>)>,
    regs: Option<Regs>,
}

struct Harness {
    cpu: CPU,
    bus: Bus,
    ctrl: Control,
    p6502: P6502,
    mem: [u8; 65536],
    pmem: [u8; 65536],
}

impl Harness {
    fn new(test_case: &TestCase) -> Self {
        let mut mem = [0u8; 65536];
        let mut pmem = [0u8; 65536];
        for (offset, bytes) in &test_case.data {
            mem[*offset..*offset + bytes.len()].copy_from_slice(&bytes);
            pmem[*offset..*offset + bytes.len()].copy_from_slice(&bytes);
        }

        let bus = if let Some(regs) = test_case.regs {
            pmem[0xfffc] = 0x00;
            pmem[0xfffd] = 0xf0;
            #[rustfmt::skip]
            pmem[0xf000..0xf010].copy_from_slice(&[
                /* LDY #y  */ 0xa0, regs.y,
                /* LDX #sp */ 0xa2, regs.sp,
                /* TXS     */ 0x9a,
                /* LDX #x  */ 0xa2, regs.x,
                /* LDA #p  */ 0xa9, regs.flags,
                /* PHA     */ 0x48,
                /* LDA #a  */ 0xa9, regs.a,
                /* PLP     */ 0x28,
                /* JMP #pc */ 0x4c, regs.pc as u8, (regs.pc >> 8) as u8
            ]);

            mem[0xfffc] = regs.pc as u8;
            mem[0xfffd] = (regs.pc >> 8) as u8;

            Bus::new(regs.pc, pmem[regs.pc as usize])
        } else {
            Bus::new(0, 0)
        };

        let cpu = CPU::new(&test_case.regs);
        let ctrl = Control::new();
        let p6502 = P6502::new();

        let mut harness = Self {
            cpu,
            bus,
            ctrl,
            p6502,
            mem,
            pmem,
        };

        // Reset perfect6502.
        for _ in 0..9 {
            harness.p_tick();
        }
        if test_case.regs.is_none() {
            // Reset r6502.
            for _ in 0..7 {
                harness.r_tick();
            }
        } else {
            // Setup initial registers for p6502.
            for _ in 0..22 {
                harness.p_tick();
            }
        }

        harness
    }

    fn r_tick(&mut self) {
        self.cpu.tick(&mut self.bus, &mut self.ctrl, nmos::exec);
        if self.bus.rw() {
            self.bus.data = self.mem[self.bus.addr as usize];
        } else {
            self.mem[self.bus.addr as usize] = self.bus.data;
        }
    }

    fn p_tick(&mut self) {
        self.p6502.step(&mut self.pmem);
        self.p6502.step(&mut self.pmem);
    }
}

#[test]
fn test_1() {
    let test_case = TestCase {
        data: vec![(0, vec![0xa9, 0xff, 0xa9, 0xaa])],
        regs: Some(Regs {
            a: 0x12,
            x: 0x34,
            y: 0x56,
            sp: 0x78,
            flags: 0b110,
            pc: 0,
        }),
    };
    let mut h = Harness::new(&test_case);

    h.p_tick();
    h.r_tick();
    println!("R {} {} {}", &h.ctrl, &h.bus, &h.cpu);
    println!("P {}", h.p6502);
    h.p_tick();
    h.r_tick();
    println!("R {} {} {}", &h.ctrl, &h.bus, &h.cpu);
    println!("P {}", h.p6502);
    h.p_tick();
    h.r_tick();
    println!("R {} {} {}", &h.ctrl, &h.bus, &h.cpu);
    println!("P {}", h.p6502);
    h.p_tick();
    h.r_tick();
    println!("R {} {} {}", &h.ctrl, &h.bus, &h.cpu);
    println!("P {}", h.p6502);
}
