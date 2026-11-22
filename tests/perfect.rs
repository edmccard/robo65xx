// TODO: change 0xab "magic constant" for tests
//       used timed6502 for 0x0b, 0x2b, 0x4b, 0x6b, 0xbb

use std::{
    ffi::{c_int, c_void},
    fmt::Display,
};

use robo65xx::*;

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
            "{} {}{}{}{} {} {:04x} {:02x} {} {:02x} {:04x} {:02x} {:02x} {:02x} {:02x} {} {}",
            if self.sync() { "S" } else { "s" },
            if self.res() { "R" } else { "r" },
            if self.nmi() { "N" } else { "n" },
            if self.irq() { "I" } else { "i" },
            if self.rdy() { "*" } else { "," },
            if self.write() { "W" } else { "R" },
            self.addr(),
            self.data(),
            if self.intg() { "!" } else { "." },
            self.op(),
            self.pc(),
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

#[derive(Clone, Copy)]
enum D {
    M(u8),
    AM(u8, u8),
    A(u8),
    X(u8),
    Y(u8),
    SP(u8),
    None,
}

type MemF = fn(u8, D, Regs) -> Vec<(usize, Vec<u8>)>;

// Values that are zero, positive and negative (in themselves, and after decrement)
const LOAD_DEC_DATA: &[D] = &[D::M(0x00), D::M(0x01), D::M(0x80)];
// Values that are zero, positive and negative (in themselves, and after increment)
const INC_DATA: &[D] = &[D::M(0x7f), D::M(0xff), D::M(0x00)];
// Stores use the default registers, with values to test izy/abi page crossing
const STORE_NOP_DATA: &[D] = &[D::None, D::Y(0x87), D::Y(0xcd), D::X(0xcd)];
// Comparisons <, =, > to the appropriate register
const CMP_DATA: &[D] = &[D::M(0x11), D::M(0x12), D::M(0x13)];
const CPX_DATA: &[D] = &[D::M(0x33), D::M(0x34), D::M(0x35)];
const CPY_DATA: &[D] = &[D::M(0x44), D::M(0x45), D::M(0x46)];

const ORA_DATA: &[D] = &[D::AM(0x00, 0x00), D::AM(0x01, 0x00), D::AM(0x80, 0x01)];
const AND_DATA: &[D] = &[D::AM(0xaa, 0x55), D::AM(0x10, 0x80), D::AM(0x80, 0xff)];
const EOR_DATA: &[D] = &[D::AM(0x33, 0x33), D::AM(0xff, 0x80), D::AM(0xff, 0x01)];
const BIT_DATA: &[D] = &[D::AM(0x00, 0xc0), D::AM(0xff, 0x3f), D::AM(0xaa, 0x55)];
const LEFT_SHIFT_DATA: &[D] = &[
    D::M(0x80),
    D::M(0x00),
    D::M(0x40),
    D::M(0xc0),
    D::M(0xa0),
    D::M(0x10),
];
const LEFT_SHIFT_A_DATA: &[D] = &[
    D::A(0x80),
    D::A(0x00),
    D::A(0x40),
    D::A(0xc0),
    D::A(0xa0),
    D::A(0x10),
];
// Values that >> to 0 or positive when carry unset or negative when carry set
const RIGHT_SHIFT_DATA: &[D] = &[D::M(0x01), D::M(0xfe)];
const RIGHT_SHIFT_A_DATA: &[D] = &[D::A(0x01), D::A(0xfe)];

const REL_DATA: &[D] = &[D::M(0xfd), D::M(0x01)];
const PULL_DATA: &[D] = &[D::M(0b00000000), D::M(0b01010101), D::M(0b10101010)];
const ADC_DATA: &[D] = &[
    D::AM(0x80, 0x80),
    D::AM(0xff, 0x02),
    D::AM(0x50, 0x50),
    D::AM(0x10, 0x20),
    D::AM(0x10, 0x90),
    D::AM(0x10, 0x70),
    D::AM(0x10, 0x10),
];
const SBC_DATA: &[D] = &[
    D::AM(0x80, 0x01),
    D::AM(0x81, 0x01),
    D::AM(0x50, 0x10),
    D::AM(0x50, 0x80),
    D::AM(0x10, 0x10),
    D::AM(0x10, 0x20),
    D::AM(0x10, 0x01),
    D::AM(0x10, 0x80),
];
const A_DATA: &[D] = &[D::A(0x00), D::A(0x01), D::A(0x80)];
const X_DATA: &[D] = &[D::X(0x00), D::X(0x01), D::X(0x80)];
const Y_DATA: &[D] = &[D::Y(0x00), D::Y(0x01), D::Y(0x80)];
const SP_DATA: &[D] = &[D::SP(0x00), D::SP(0x01), D::SP(0x80)];
const INX_DATA: &[D] = &[D::X(0x7f), D::X(0xff), D::X(0x00)];
const INY_DATA: &[D] = &[D::Y(0x7f), D::Y(0xff), D::Y(0x00)];
const NO_DATA: &[D] = &[D::None];

fn run_test(opcode: u8) -> bool {
    let flags: &[u8] = match opcode {
        0x08 | 0x18 | 0x38 | 0x48 | 0x58 | 0x78 | 0xb8 | 0xd8 | 0xf8 => {
            // Push and flag ops
            &[0b00000000, 0b11111111, 0b01010101, 0b10101010]
        }
        0x61 | 0x65 | 0x69 | 0x71 | 0x75 | 0xcb | 0xe1 | 0xe5 | 0xe9 | 0xeb | 0xf1 | 0xf5
        | 0x6d | 0x79 | 0x7d | 0xed | 0xf9 | 0xfd => {
            // Decimal mode ops
            &[0b00000000, 0b11110111, 0b00001000, 0b11111111]
        }
        _ => &[0b00000000, 0b11111111],
    };

    let (data, memf) = match opcode {
        0xa1 | 0xa5 | 0xa9 | 0xb1 | 0xb5 | 0xa2 | 0xa6 | 0xb6 | 0xa0 | 0xa4 | 0xb4 | 0xa3
        | 0xa7 | 0xb3 | 0xb7 | 0x8b | 0xc6 | 0xd6 | 0xc7 | 0xd7 | 0xc3 | 0xd3 => {
            (LOAD_DEC_DATA, mem_two_byte as MemF)
        }
        0xad | 0xbd | 0xb9 | 0xae | 0xbe | 0xac | 0xbc | 0xaf | 0xbf | 0xce | 0xcf | 0xde
        | 0xdf | 0xdb => (LOAD_DEC_DATA, mem_three_byte as MemF),
        0x04 | 0x14 | 0x34 | 0x44 | 0x54 | 0x64 | 0x74 | 0x80 | 0x82 | 0x89 | 0xc2 | 0xe2
        | 0xd4 | 0xf4 | 0x81 | 0x83 | 0x84 | 0x85 | 0x86 | 0x87 | 0x91 | 0x93 | 0x94 | 0x95
        | 0x96 | 0x97 => (STORE_NOP_DATA, mem_two_byte as MemF),
        0x0c | 0x1c | 0x3c | 0x5c | 0x7c | 0xdc | 0xfc | 0x8c | 0x8d | 0x8e | 0x8f | 0x99
        | 0x9b | 0x9c | 0x9d | 0x9e | 0x9f => (STORE_NOP_DATA, mem_three_byte as MemF),
        0xc1 | 0xc5 | 0xc9 | 0xd1 | 0xd5 => (CMP_DATA, mem_two_byte as MemF),
        0xcd | 0xd9 | 0xdd => (CMP_DATA, mem_three_byte as MemF),
        0xe0 | 0xe4 => (CPX_DATA, mem_two_byte as MemF),
        0xec => (CPX_DATA, mem_three_byte as MemF),
        0xc0 | 0xc4 => (CPY_DATA, mem_two_byte as MemF),
        0xcc => (CPY_DATA, mem_three_byte as MemF),
        0x46 | 0x56 | 0x66 | 0x76 | 0x27 | 0x37 | 0x23 | 0x33 | 0x67 | 0x77 | 0x63 | 0x73 => {
            (RIGHT_SHIFT_DATA, mem_two_byte as MemF)
        }
        0x4e | 0x4f | 0x5e | 0x6e | 0x6f | 0x7e | 0x3f | 0x3b | 0x7f | 0x7b => {
            (RIGHT_SHIFT_DATA, mem_three_byte as MemF)
        }
        0x06 | 0x16 | 0x26 | 0x36 | 0x07 | 0x17 | 0x03 | 0x13 | 0x47 | 0x57 | 0x43 | 0x53 => {
            (LEFT_SHIFT_DATA, mem_two_byte as MemF)
        }
        0x0e | 0x0f | 0x1e | 0x2e | 0x2f | 0x3e | 0x1f | 0x1b | 0x5f | 0x5b => {
            (LEFT_SHIFT_DATA, mem_three_byte as MemF)
        }
        0xe6 | 0xf6 | 0xe7 | 0xf7 | 0xe3 | 0xf3 => (INC_DATA, mem_two_byte as MemF),
        0xee | 0xef | 0xfe | 0xff | 0xfb => (INC_DATA, mem_three_byte as MemF),
        0x01 | 0x05 | 0x09 | 0x11 | 0x15 => (ORA_DATA, mem_two_byte as MemF),
        0x0d | 0x19 | 0x1d => (ORA_DATA, mem_three_byte as MemF),
        0x21 | 0x25 | 0x29 | 0x31 | 0x35 => (AND_DATA, mem_two_byte as MemF),
        0x2d | 0x39 | 0x3d => (AND_DATA, mem_three_byte as MemF),
        0x41 | 0x45 | 0x49 | 0x51 | 0x55 => (EOR_DATA, mem_two_byte as MemF),
        0x4d | 0x59 | 0x5d => (EOR_DATA, mem_three_byte as MemF),
        0x24 => (BIT_DATA, mem_two_byte as MemF),
        0x2c => (BIT_DATA, mem_three_byte as MemF),
        0xba => (SP_DATA, mem_one_byte as MemF),
        0xa8 | 0xaa => (A_DATA, mem_one_byte as MemF),
        0xca | 0x8a => (X_DATA, mem_one_byte as MemF),
        0x88 | 0x98 => (Y_DATA, mem_one_byte as MemF),
        0xe8 => (INX_DATA, mem_one_byte as MemF),
        0xc8 => (INY_DATA, mem_one_byte as MemF),
        0x0a | 0x2a => (LEFT_SHIFT_A_DATA, mem_one_byte as MemF),
        0x4a | 0x6a => (RIGHT_SHIFT_A_DATA, mem_one_byte as MemF),
        0x08 | 0x18 | 0x38 | 0x48 | 0x58 | 0x78 | 0xb8 | 0xd8 | 0xf8 | 0x1a | 0x3a | 0x5a
        | 0x7a | 0x9a | 0xda | 0xea | 0xfa => (NO_DATA, mem_one_byte as MemF),
        0x10 | 0x30 | 0x50 | 0x70 | 0x90 | 0xb0 | 0xd0 | 0xf0 => (REL_DATA, mem_branch as MemF),
        0x28 | 0x68 => (PULL_DATA, mem_pull as MemF),
        0x61 | 0x65 | 0x69 | 0x71 | 0x75 => (ADC_DATA, mem_two_byte as MemF),
        0x6d | 0x79 | 0x7d => (ADC_DATA, mem_three_byte as MemF),
        0xcb | 0xe1 | 0xe5 | 0xe9 | 0xeb | 0xf1 | 0xf5 => (SBC_DATA, mem_two_byte as MemF),
        0xed | 0xf9 | 0xfd => (SBC_DATA, mem_three_byte as MemF),
        _ => return false,
    };

    for &flag in flags {
        for &d in data {
            let regs = regs_data(flag, d);
            let tc = TestCase {
                mem: memf(opcode, d, regs),
                regs: Some(regs),
            };
            let mut h = Harness::new(&tc);
            h.run();
        }
    }

    true
}

fn regs_default() -> Regs {
    Regs {
        pc: 0x200,
        a: 0x12,
        x: 0x34,
        y: 0x45,
        sp: 0x56,
        flags: 0b110,
    }
}

fn regs_data(flags: u8, data: D) -> Regs {
    match data {
        D::AM(a, _) => Regs {
            flags,
            a,
            ..regs_default()
        },
        D::M(_) | D::None => Regs {
            flags,
            ..regs_default()
        },
        D::A(a) => Regs {
            flags,
            a,
            ..regs_default()
        },
        D::X(x) => Regs {
            flags,
            x,
            ..regs_default()
        },
        D::Y(y) => Regs {
            flags,
            y,
            ..regs_default()
        },
        D::SP(sp) => Regs {
            flags,
            sp,
            ..regs_default()
        },
    }
}

fn mem_one_byte(opcode: u8, _: D, _: Regs) -> Vec<(usize, Vec<u8>)> {
    vec![(0x200, vec![opcode])]
}

fn mem_branch(opcode: u8, data: D, _: Regs) -> Vec<(usize, Vec<u8>)> {
    let offset = match data {
        D::M(o) => o,
        _ => unreachable!(),
    };
    vec![(0x200, vec![opcode, offset, 0xea])]
}

fn mem_pull(opcode: u8, data: D, _: Regs) -> Vec<(usize, Vec<u8>)> {
    let stack = match data {
        D::M(s) => s,
        _ => unreachable!(),
    };
    vec![(0x200, vec![opcode]), (0x156, vec![stack])]
}

fn mem_two_byte(opcode: u8, data: D, regs: Regs) -> Vec<(usize, Vec<u8>)> {
    assert!(regs.x.abs_diff(regs.y) > 2 && regs.x > 1 && regs.y > 1);
    let vals = match data {
        D::M(v) | D::AM(_, v) => [v; 6],
        _ => {
            let v = [0x78, 0x79, 0x80, 0x81, 0x82, 0x83];
            for byte in v {
                assert!(regs.a != byte && regs.x != byte && regs.y != byte);
            }
            v
        }
    };
    vec![
        // immediate value / zp addr / zpi base
        (regs.pc as usize, vec![opcode, vals[0]]),
        // zp value / izy base
        (vals[0] as usize, vec![vals[1], 0x50]), // if izy, call with y < 0x87 and y >= 0x87
        // zpx value / izx addr lo
        (vals[0].wrapping_add(regs.x) as usize, vec![vals[2]]),
        // izx addr hi
        (
            vals[0].wrapping_add(regs.x).wrapping_add(1) as usize,
            vec![0x60],
        ),
        // zpy value
        (vals[0].wrapping_add(regs.y) as usize, vec![vals[3]]),
        // izx value
        (0x6080, vec![vals[4]]),
        // izy value
        (
            0x5079u16.wrapping_add(regs.y as u16) as usize,
            vec![vals[5]],
        ),
    ]
}

fn mem_three_byte(opcode: u8, data: D, regs: Regs) -> Vec<(usize, Vec<u8>)> {
    let vals = match data {
        D::M(v) | D::AM(_, v) => [v; 3],
        _ => {
            let v = [0x78, 0x79, 0x80];
            for byte in v {
                assert!(regs.a != byte && regs.x != byte && regs.y != byte);
            }
            v
        }
    };
    vec![
        (regs.pc as usize, vec![opcode, 0x33, 0x44]),
        (0x4433, vec![vals[0]]), // abs value
        // if abi, call with x/y < 0xcd and x/y >= cd
        (
            0x4433u16.wrapping_add(regs.x as u16) as usize,
            vec![vals[1]],
        ), // abx value
        (
            0x4433u16.wrapping_add(regs.y as u16) as usize,
            vec![vals[2]],
        ), // aby value
    ]
}

struct TestCase {
    mem: Vec<(usize, Vec<u8>)>,
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
        for (offset, bytes) in &test_case.mem {
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
            harness.pmem[0x100..0x200].copy_from_slice(&mem[0x100..0x200]);
            harness.pmem[0xf000..0xf010].copy_from_slice(&mem[0xf000..0xf010]);
            harness.pmem[0xfffc] = mem[0xfffc];
            harness.pmem[0xfffd] = mem[0xfffd];
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

    fn run(&mut self) {
        let mut p_log = Vec::<String>::new();
        let mut log = Vec::<String>::new();
        loop {
            self.p_tick();
            p_log.push(format!("{}", self.p6502)[0..47].into());
            if p_log.len() > 99 {
                assert!(false, "perfect6502 ran too long");
            }
            if self.p6502.op() == 0x00 {
                break;
            }
        }
        loop {
            self.r_tick();
            log.push(format!("{} {} {}", self.ctrl, self.bus, self.cpu)[0..47].into());
            if self.cpu.op() == 0x00 || log.len() > p_log.len() {
                break;
            }
        }
        if p_log.len() > log.len() {
            // interrupt too soon, or single op wrong cycle count
            let mut msg = String::new();
            for i in 0..p_log.len() {
                let prefix = if i < log.len() { " " } else { "-" };
                msg.push_str(&format!("{}{:2} {}\n   ", prefix, i, &p_log[i][0..21]));
            }
            assert!(false, "{}", msg);
        } else if p_log.len() < log.len() {
            // interrupt too late, or single op wrong cycle count
            let mut msg = String::new();
            for i in 0..log.len() {
                let prefix = if i >= p_log.len() { "+" } else { " " };
                msg.push_str(&format!("{}{:2} {}\n   ", prefix, i, &log[i][0..21]));
            }
            assert!(false, "{}", msg);
        }
        if !Harness::check_bus(&p_log, &log) {
            let mut msg = String::new();
            for i in 0..p_log.len() {
                msg.push_str(&format!("{:2} {}\n   ", i, &p_log[i][0..21]));
                msg.push_str(&Harness::log_diff(&p_log[i][0..21], &log[i][0..21]))
            }
            assert!(false, "{}", msg);
        }
        let p_regs = &p_log.last().unwrap()[19..47];
        let regs = &log.last().unwrap()[19..47];
        let op = &log[log.len() - 2][19..21];
        if p_regs != regs {
            let i = log.len() - 1;
            let mut msg = format!("{:2} {} {}\n   ", i, op, &p_regs[3..]);
            msg.push_str(&Harness::log_diff(p_regs, regs));
            assert!(false, "{}", msg);
        }
    }

    fn log_diff(p: &str, r: &str) -> String {
        let mut diff = String::new();
        diff.push_str(
            &p.chars()
                .zip(r.chars())
                .map(|(c1, c2)| if c1 == c2 { ' ' } else { c2 })
                .collect::<String>(),
        );
        diff.push('\n');
        diff
    }

    fn check_bus(p_log: &[String], log: &[String]) -> bool {
        for i in 0..p_log.len() {
            if p_log[i][0..21] != log[i][0..21] {
                return false;
            }
        }
        true
    }
}

#[test]
fn test_reset() {
    let test_case = TestCase {
        mem: vec![(0x200, vec![0xa9]), (0xfffc, vec![0x00, 0x02])],
        regs: None,
    };
    let h = Harness::new(&test_case);
    assert_eq!(
        format!("{}", h.p6502)[0..47],
        format!("{} {} {}", &h.ctrl, &h.bus, &h.cpu)[0..47]
    );
}

#[test]
fn test_flow() {
    // JMP
    let tc = TestCase {
        mem: vec![(0x200, vec![0x4c, 0x10, 0x30]), (0x3010, vec![0xea])],
        regs: Some(Regs { ..regs_default() }),
    };
    let mut h = Harness::new(&tc);
    h.run();

    // JMP indirect
    for lo_byte in [0x10, 0xff] {
        let tc = TestCase {
            mem: vec![
                (0x200, vec![0x6c, lo_byte, 0x30]),
                (0x3010, vec![0x20, 0x40]),
                (0x30ff, vec![0x60]),
                (0x3100, vec![0x60]),
                (0x3000, vec![0x50]),
                (0x5060, vec![0xea]),
                (0x6060, vec![0xea]),
                (0x4020, vec![0xea]),
            ],
            regs: Some(Regs { ..regs_default() }),
        };
        let mut h = Harness::new(&tc);
        h.run();
    }

    // JSR / RTS
    let tc = TestCase {
        mem: vec![(0x200, vec![0x20, 0x10, 0x30, 0xea]), (0x3010, vec![0x60])],
        regs: Some(Regs { ..regs_default() }),
    };
    let mut h = Harness::new(&tc);
    h.run();
}

#[test]
fn test_all() {
    for opcode in 0..=255 {
        if !run_test(opcode) {
            println!("SKIPPED {:02x}", opcode);
        }
    }
}
