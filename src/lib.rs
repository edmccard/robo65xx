use std::fmt::Display;

mod ops;

#[derive(Default)]
pub struct Bus {
    pub addr: u16,
    pub data: u8,
    write: bool,
}

impl Bus {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn rw(&self) -> bool {
        !self.write
    }

    fn read(&mut self, addr: u16) {
        self.addr = addr;
        self.write = false;
    }

    fn write(&mut self, addr: u16, data: u8) {
        self.addr = addr;
        self.data = data;
        self.write = true;
    }
}

impl Display for Bus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {:04x} {:02x}",
            if self.write { "W" } else { "R" },
            self.addr,
            self.data
        )
    }
}

#[derive(Clone, Copy, Default)]
pub struct Control {
    sync: bool,
    pub irq: bool,
    pub nmi: bool,
    pub res: bool,
}

impl Control {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn init() -> Self {
        Self {
            sync: true,
            res: true,
            ..Default::default()
        }
    }

    pub fn sync(&self) -> bool {
        self.sync
    }
}

impl Display for Control {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}{}{}",
            if self.sync { "S" } else { "s" },
            if self.res { "R" } else { "r" },
            if self.nmi { "N" } else { "n" },
            if self.irq { "I" } else { "i" },
        )
    }
}

#[derive(Default)]
pub struct Flags(pub u8);

impl Flags {
    pub fn n(&self) -> bool {
        self.0 & 0b10000000 != 0
    }

    fn set_n(&mut self, val: u8) {
        self.0 &= val & 0b01111111;
        self.0 |= val & 0b10000000;
    }

    pub fn v(&self) -> bool {
        self.0 & 0b01000000 != 0
    }

    fn update_v(&mut self, bit: bool) {
        if bit {
            self.0 |= 0b01000000;
        } else {
            self.0 &= 0b10111111;
        }
    }

    pub fn d(&self) -> bool {
        self.0 & 0b00001000 != 0
    }

    pub fn i(&self) -> bool {
        self.0 & 0b00000100 != 0
    }

    fn update_i(&mut self, bit: bool) {
        if bit {
            self.0 |= 0b00000100;
        } else {
            self.0 &= 0b11111011;
        }
    }

    pub fn z(&self) -> bool {
        self.0 & 0b00000010 != 0
    }

    fn set_z(&mut self, val: u8) {
        if val == 0 {
            self.0 |= 0b00000010;
        } else {
            self.0 &= 0b11111101;
        }
    }

    pub fn c(&self) -> bool {
        self.0 & 0b00000001 != 0
    }

    fn update_c(&mut self, bit: bool) {
        if bit {
            self.0 |= 0b00000001;
        } else {
            self.0 &= 0b11111110;
        }
    }

    fn set_nz(&mut self, val: u8) {
        self.0 &= val & 0b01111101;
        self.0 |= val & 0b10000000;
        if val == 0 {
            self.0 |= 0b00000010;
        }
    }

    fn set_from(&mut self, val: u8, mask: u8) {
        self.0 = self.0 & !mask | val & mask | 0b00110000;
    }
}

impl Display for Flags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}--{}{}{}{}",
            if self.n() { "N" } else { "n" },
            if self.v() { "V" } else { "v" },
            if self.d() { "D" } else { "d" },
            if self.i() { "I" } else { "i" },
            if self.z() { "Z" } else { "z" },
            if self.c() { "C" } else { "c" },
        )
    }
}

pub struct CPU {
    // The current cycle of the current instruction.
    op_cycle: u64,
    // The program counter.
    pc: u16,
    // The target for a jump.
    vector: u16,
    // The CPU flags.
    flags: Flags,
    // The accumulator.
    a: u8,
    // The X-register.
    x: u8,
    // The Y-register.
    y: u8,
    // The stack pointer.
    sp: u8,
    // The instruction register.
    op: u8,
    // True if irq was active on the previous cycle
    irq_pending: bool,
    // True if nmi has crossed from inactive to active.
    nmi_pending: bool,
    // True if an interrupt should be taken.
    intg: bool,
}

impl Display for CPU {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let intg = if self.intg { "!" } else { "." };
        write!(
            f,
            "{} {} {:04x} {:02x} {:02x} {:02x} {:02x} {:02x} {}",
            intg, self.op_cycle, self.pc, self.op, self.a, self.x, self.y, self.sp, self.flags
        )
    }
}

impl CPU {
    pub fn init(bus: &Bus, ctrl: &mut Control) -> Self {
        let mut cpu = Self {
            pc: bus.addr,
            flags: Flags(0x2),
            op_cycle: 0,
            vector: 0,
            a: 0,
            x: 0,
            y: 0,
            sp: 0xc0,
            op: 0,
            irq_pending: false,
            nmi_pending: false,
            intg: false,
        };
        if ctrl.res {
            cpu.intg = true;
        } else {
            cpu.flags.update_i(true);
            cpu.sp = cpu.sp.wrapping_sub(3);
        }
        ctrl.sync = true;
        cpu
    }

    pub fn tick(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if ctrl.sync {
            let op = bus.data;
            if self.intg {
                self.op = 0x00;
            } else {
                self.op = op;
                self.pc = self.pc.wrapping_add(1);
            }
            self.op_cycle = 0;
            ctrl.sync = false;
        } else {
            self.op_cycle += 1;
        }

        self.exec(bus, ctrl);

        self.irq_pending = ctrl.irq;
        if ctrl.nmi {
            self.nmi_pending = true;
        }
    }

    fn poll_intg(&mut self, ctrl: &Control) {
        if ctrl.res || self.nmi_pending || (self.irq_pending && !self.flags.i()) {
            self.intg = true;
        }
    }

    // Read from and increment the program counter.
    fn read_pc(&mut self, bus: &mut Bus) {
        bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
    }

    // Read from the stack.
    fn read_sp(&mut self, bus: &mut Bus) {
        bus.read(self.sp as u16 | 0x100);
    }

    // Write to the stack and decrement the stack pointer.
    fn write_sp(&mut self, bus: &mut Bus, data: u8) {
        bus.write(self.sp as u16 | 0x100, data);
        self.sp = self.sp.wrapping_sub(1);
    }

    // Set up opcode fetch
    fn fetch(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        bus.read(self.pc);
        ctrl.sync = true;
    }
}
