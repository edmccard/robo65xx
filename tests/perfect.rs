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

enum Reg {
    A,
    X,
    Y,
    SP,
}

struct TestCase {
    data: Vec<(usize, Vec<u8>)>,
    regs: Option<Regs>,
}

impl TestCase {
    fn one_byte_no_deps(opcode: u8, flags: u8) -> TestCase {
        let data = vec![(0x200, vec![opcode])];
        TestCase {
            data,
            regs: Some(Regs {
                flags,
                ..test_regs()
            }),
        }
    }

    fn one_byte_reg(opcode: u8, reg: Reg, val: u8, flags: u8) -> TestCase {
        let data = vec![(0x200, vec![opcode])];
        let regs = match reg {
            Reg::A => Regs {
                flags,
                a: val,
                ..test_regs()
            },
            Reg::X => Regs {
                flags,
                x: val,
                ..test_regs()
            },
            Reg::Y => Regs {
                flags,
                y: val,
                ..test_regs()
            },
            Reg::SP => Regs {
                flags,
                sp: val,
                ..test_regs()
            },
        };
        TestCase {
            data,
            regs: Some(regs),
        }
    }

    fn two_byte_vals(opcode: u8, regs: Regs, vals: [u8; 6]) -> TestCase {
        assert!(regs.x.abs_diff(regs.y) > 2 && regs.x > 1 && regs.y > 1);
        let data = vec![
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
        ];
        TestCase {
            data,
            regs: Some(regs),
        }
    }

    fn two_byte_no_deps(opcode: u8, regs: Regs) -> TestCase {
        let vals = [0x78, 0x79, 0x80, 0x81, 0x82, 0x83];
        for byte in vals {
            assert!(regs.a != byte && regs.x != byte && regs.y != byte);
        }
        TestCase::two_byte_vals(opcode, regs, vals)
    }

    fn two_byte_val(opcode: u8, val: u8, regs: Regs) -> TestCase {
        TestCase::two_byte_vals(opcode, regs, [val; 6])
    }

    fn three_byte_vals(opcode: u8, regs: Regs, vals: [u8; 3]) -> TestCase {
        let data = vec![
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
        ];
        TestCase {
            data,
            regs: Some(regs),
        }
    }

    fn three_byte_no_deps(opcode: u8, regs: Regs) -> TestCase {
        let vals = [0x78, 0x79, 0x80];
        for byte in vals {
            assert!(regs.a != byte && regs.x != byte && regs.y != byte);
        }
        TestCase::three_byte_vals(opcode, regs, vals)
    }

    fn three_byte_val(opcode: u8, val: u8, regs: Regs) -> TestCase {
        TestCase::three_byte_vals(opcode, regs, [val; 3])
    }
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
            assert!(false, "Too few cycles");
        } else if p_log.len() < log.len() {
            // interrupt too late, or single op wrong cycle count
            assert!(false, "Too many cycles");
        }
        if !Harness::check_bus(&p_log, &log) {
            let mut msg = String::new();
            for i in 0..p_log.len() {
                msg.push_str(&format!("{:2} {}\n   ", i, &p_log[i][0..26]));
                msg.push_str(&Harness::log_diff(&p_log[i][0..26], &log[i][0..26]))
            }
            assert!(false, "{}", msg);
        }
        let p_regs = &p_log.last().unwrap()[24..47];
        let regs = &log.last().unwrap()[24..47];
        if p_regs != regs {
            let i = log.len() - 1;
            let op = &p_log[log.len() - 2][24..26];
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
            if p_log[i][0..26] != log[i][0..26] {
                return false;
            }
        }
        true
    }
}

fn test_regs() -> Regs {
    Regs {
        pc: 0x200,
        a: 0x12,
        x: 0x34,
        y: 0x45,
        sp: 0x56,
        flags: 0b110,
    }
}

#[test]
fn test_reset() {
    let test_case = TestCase {
        data: vec![(0x200, vec![0xa9]), (0xfffc, vec![0x00, 0x02])],
        regs: None,
    };
    let h = Harness::new(&test_case);
    assert_eq!(
        format!("{}", h.p6502)[0..47],
        format!("{} {} {}", &h.ctrl, &h.bus, &h.cpu)[0..47]
    );
}

#[test]
fn test_flags_push() {
    //             PHP   CLC   SEC   PHA   CLI   SEI   CLV   CLD   SED
    for opcode in [0x08, 0x18, 0x38, 0x48, 0x58, 0x78, 0xb8, 0xd8, 0xf8] {
        for flags in [0b00000000, 0b11111111, 0b01010101, 0b10101010] {
            let tc = TestCase::one_byte_no_deps(opcode, flags);
            let mut h = Harness::new(&tc);
            h.run();
        }
    }
}

#[test]
fn test_pull() {
    //             PLP   PLA
    for opcode in [0x28, 0x68] {
        for flags in [0b00000000, 0b11111111] {
            for val in [0b00000000, 0b01010101, 0b10101010] {
                let tc = TestCase {
                    data: vec![(0x200, vec![opcode]), (0x156, vec![val])],
                    regs: Some(Regs {
                        flags: flags,
                        ..test_regs()
                    }),
                };
                let mut h = Harness::new(&tc);
                h.run();
            }
        }
    }
}

#[test]
fn test_implicit_no_deps() {
    //             NOP   NOP   NOP   NOP   TXS   NOP   NOP   NOP
    for opcode in [0x1a, 0x3a, 0x5a, 0x7a, 0x9a, 0xda, 0xea, 0xfa] {
        for flags in [0b00000000, 0b11111111] {
            let tc = TestCase::one_byte_no_deps(opcode, flags);
            let mut h = Harness::new(&tc);
            h.run();
        }
    }
}

#[test]
fn test_implicit_nz() {
    for val in [0x00, 0x01, 0x80] {
        for flags in [0b00000000, 0b11111111] {
            // TSX
            let tc = TestCase::one_byte_reg(0xba, Reg::SP, val, flags);
            let mut h = Harness::new(&tc);
            h.run();
            //             TAY   TAX
            for opcode in [0xa8, 0xaa] {
                let tc = TestCase::one_byte_reg(opcode, Reg::A, val, flags);
                let mut h = Harness::new(&tc);
                h.run();
            }
            //             DEX   TXA
            for opcode in [0xca, 0x8a] {
                let tc = TestCase::one_byte_reg(opcode, Reg::X, val, flags);
                let mut h = Harness::new(&tc);
                h.run();
            }
            //             DEY   TYA
            for opcode in [0x88, 0x98] {
                let tc = TestCase::one_byte_reg(opcode, Reg::Y, val, flags);
                let mut h = Harness::new(&tc);
                h.run();
            }
        }
    }
    for val in [0x7f, 0xff, 0x00] {
        for flags in [0b00000000, 0b11111111] {
            // INX
            let tc = TestCase::one_byte_reg(0xe8, Reg::X, val, flags);
            let mut h = Harness::new(&tc);
            h.run();
            // INY
            let tc = TestCase::one_byte_reg(0xc8, Reg::Y, val, flags);
            let mut h = Harness::new(&tc);
            h.run();
        }
    }
}

#[test]
fn test_two_byte_no_deps() {
    #[rustfmt::skip]
    let opcodes = [
    //  NOP
        0x04, 0x14, 0x34, 0x44, 0x54, 0x64, 0x74, 0x80, 0x82, 0x89, 0xc2, 0xe2, 0xd4, 0xf4,
    //  STA   SAX   STY   STA   STX   SAX   STA   AHX   STY   STA   STX   SAX
        0x81, 0x83, 0x84, 0x85, 0x86, 0x87, 0x91, 0x93, 0x94, 0x95, 0x96, 0x97,
    ];

    for opcode in opcodes {
        for flags in [0b00000000, 0b11111111] {
            let tc = TestCase::two_byte_no_deps(
                opcode,
                Regs {
                    flags: flags,
                    ..test_regs()
                },
            );
            let mut h = Harness::new(&tc);
            h.run();
            let tc = TestCase::two_byte_no_deps(
                opcode,
                Regs {
                    flags: flags,
                    y: 0x87, // for izy page crossing
                    ..test_regs()
                },
            );
            let mut h = Harness::new(&tc);
            h.run();
        }
    }
}

#[test]
fn test_two_byte_nz() {
    #[rustfmt::skip]
    let opcodes = [
    //  LDA
        0xa1, 0xa5, 0xa9, 0xb1, 0xb5,
    //  LDX               LDY
        0xa2, 0xa6, 0xb6, 0xa0, 0xa4, 0xb4,
    //  LAX
        0xa3, 0xa7, 0xb3, 0xb7,
    //  DEC
        0xc6, 0xd6,
    ];

    for opcode in opcodes {
        for val in [0x00, 0x01, 0x80] {
            for flags in [0b00000000, 0b11111111] {
                let tc = TestCase::two_byte_val(
                    opcode,
                    val,
                    Regs {
                        flags,
                        ..test_regs()
                    },
                );
                let mut h = Harness::new(&tc);
                h.run();
            }
        }
    }

    // INC
    for opcode in [0xe6, 0xf6] {
        for val in [0x7f, 0xff, 0x00] {
            for flags in [0b00000000, 0b11111111] {
                let tc = TestCase::two_byte_val(
                    opcode,
                    val,
                    Regs {
                        flags,
                        ..test_regs()
                    },
                );
                let mut h = Harness::new(&tc);
                h.run();
            }
        }
    }

    let booleans = [
        //  ORA
        (
            vec![0x01, 0x05, 0x09, 0x11, 0x15],
            vec![(0x00, 0x00), (0x01, 0x00), (0x80, 0x01)],
        ),
        //  AND
        (
            vec![0x21, 0x25, 0x29, 0x31, 0x35],
            vec![(0xaa, 0x55), (0x10, 0x80), (0x80, 0xff)],
        ),
        //  EOR
        (
            vec![0x41, 0x45, 0x49, 0x41, 0x55],
            vec![(0x33, 0x33), (0xff, 0x80), (0xff, 0x01)],
        ),
        // BIT
        (vec![0x24], vec![(0x00, 0xc0), (0xff, 0x3f), (0xaa, 0x55)]),
    ];

    for flags in [0b00000000, 0b11111111] {
        for (opcodes, vals) in &booleans {
            for &opcode in opcodes {
                for &val in vals {
                    let tc = TestCase::two_byte_val(
                        opcode as u8,
                        val.1 as u8,
                        Regs {
                            flags,
                            a: val.0,
                            ..test_regs()
                        },
                    );
                    let mut h = Harness::new(&tc);
                    h.run();
                }
            }
        }
    }
}

#[test]
fn test_three_byte_no_deps() {
    #[rustfmt::skip]
    let opcodes = [
    //  NOP
        0x0c, 0x1c, 0x3c, 0x5c, 0x7c, 0xdc, 0xfc,
    //  STY   STA   STX   SAX   TAS   SHY   STA   SHX   AHX   LAS
        0x8c, 0x8d, 0x8e, 0x8f, 0x9b, 0x9c, 0x9d, 0x9e, 0x9f,
    ];

    for opcode in opcodes {
        for flags in [0b00000000, 0b11111111] {
            let tc = TestCase::three_byte_no_deps(
                opcode,
                Regs {
                    flags,
                    ..test_regs()
                },
            );
            let mut h = Harness::new(&tc);
            h.run();
            let tc = TestCase::three_byte_no_deps(
                opcode,
                Regs {
                    flags,
                    x: 0xcd,
                    y: 0xcd,
                    ..test_regs()
                },
            );
            let mut h = Harness::new(&tc);
            h.run();
        }
    }
}

#[test]
fn test_three_byte_nz() {
    #[rustfmt::skip]
    let opcodes = [
    //  LDA
        0xad, 0xbd, 0xb9,
    //  LDX         LDY
        0xae, 0xbe, 0xac, 0xbc,
    //  LAX
        0xaf, 0xbf,
    //  DEC
        0xce, 0xde,
    ];

    for opcode in opcodes {
        for val in [0x00, 0x01, 0x80] {
            for flags in [0b00000000, 0b11111111] {
                let tc = TestCase::three_byte_val(
                    opcode,
                    val,
                    Regs {
                        flags,
                        ..test_regs()
                    },
                );
                let mut h = Harness::new(&tc);
                h.run();
            }
        }
    }

    // INC
    for opcode in [0xee, 0xfe] {
        for val in [0x7f, 0xff, 0x00] {
            for flags in [0b00000000, 0b11111111] {
                let tc = TestCase::three_byte_val(
                    opcode,
                    val,
                    Regs {
                        flags,
                        ..test_regs()
                    },
                );
                let mut h = Harness::new(&tc);
                h.run();
            }
        }
    }

    let booleans = [
        //  ORA
        (
            vec![0x0d, 0x19, 0x1d],
            vec![(0x00, 0x00), (0x01, 0x00), (0x80, 0x01)],
        ),
        //  AND
        (
            vec![0x2d, 0x39, 0x3d],
            vec![(0xaa, 0x55), (0x10, 0x80), (0x80, 0xff)],
        ),
        //  EOR
        (
            vec![0x4d, 0x59, 0x5d],
            vec![(0x33, 0x33), (0xff, 0x80), (0xff, 0x01)],
        ),
        // BIT
        (vec![0x2c], vec![(0x00, 0xc0), (0xff, 0x3f), (0xaa, 0x55)]),
    ];

    for flags in [0b00000000, 0b11111111] {
        for (opcodes, vals) in &booleans {
            for &opcode in opcodes {
                for &val in vals {
                    let tc = TestCase::three_byte_val(
                        opcode as u8,
                        val.1 as u8,
                        Regs {
                            flags,
                            a: val.0,
                            ..test_regs()
                        },
                    );
                    let mut h = Harness::new(&tc);
                    h.run();
                }
            }
        }
    }
}
