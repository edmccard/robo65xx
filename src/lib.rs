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

fn add_without_carry(a: u16, b: u8) -> u16 {
    a.wrapping_add(b as u16) & 0xff00
}

// Addressing
impl CPU {
    fn implicit<F>(&mut self, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self),
    {
        match self.op_cycle {
            0 => {
                bus.read(self.pc); // dummy read
                self.poll_intg(ctrl);
            }
            1 => {
                op(self);
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn imm_alu<F>(&mut self, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8),
    {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // operand
                self.poll_intg(ctrl);
            }
            1 => {
                op(self, bus.data);
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn zp_alu<F>(&mut self, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8),
    {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // zero-page address
            }
            1 => {
                bus.read(bus.data as u16); // operand
                self.poll_intg(ctrl);
            }
            2 => {
                op(self, bus.data);
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn zp_store(&mut self, src: u8, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // zero-page address
            }
            1 => {
                bus.write(bus.data as u16, src);
                self.poll_intg(ctrl);
            }
            2 => {
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn zp_rmw<F>(&mut self, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8) -> u8,
    {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // zero-page address
            }
            1 => {
                bus.read(bus.data as u16); // operand
            }
            2 => {
                bus.write(bus.addr, bus.data); // dummy write
            }
            3 => {
                bus.write(bus.addr, op(self, bus.data));
                self.poll_intg(ctrl);
            }
            4 => {
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn abs_alu<F>(&mut self, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8),
    {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // low byte of address
            }
            1 => {
                self.vector = bus.data as u16;
                self.read_pc(bus); // high byte of address
            }
            2 => {
                bus.read((bus.data as u16).wrapping_shl(8) | self.vector); // operand
                self.poll_intg(ctrl);
            }
            3 => {
                op(self, bus.data);
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn abs_store(&mut self, src: u8, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // low byte of address
            }
            1 => {
                self.vector = bus.data as u16;
                self.read_pc(bus); // high byte of address
            }
            2 => {
                bus.write((bus.data as u16).wrapping_shl(8) | self.vector, src);
                self.poll_intg(ctrl);
            }
            3 => {
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn abs_rmw<F>(&mut self, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8) -> u8,
    {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // low byte of address
            }
            1 => {
                self.vector = bus.data as u16;
                self.read_pc(bus); // high byte of address
            }
            2 => {
                bus.read((bus.data as u16).wrapping_shl(8) | self.vector); // operand
            }
            3 => {
                bus.write(bus.addr, bus.data); // dummy write
            }
            4 => {
                bus.write(bus.addr, op(self, bus.data));
                self.poll_intg(ctrl);
            }
            5 => {
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn zpi_alu<F>(&mut self, reg: u8, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8),
    {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // base zero-page address
            }
            1 => {
                self.vector = add_without_carry(bus.data as u16, reg);
                bus.read(bus.data as u16); // dummy read
            }
            2 => {
                bus.read(self.vector); // operand
                self.poll_intg(ctrl);
            }
            3 => {
                op(self, bus.data);
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn zpi_store(&mut self, reg: u8, src: u8, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // base zero-page address
            }
            1 => {
                self.vector = add_without_carry(bus.data as u16, reg);
                bus.read(bus.data as u16); // dummy read
            }
            2 => {
                bus.write(self.vector, src); // operand
                self.poll_intg(ctrl);
            }
            3 => {
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn zpi_rmw<F>(&mut self, reg: u8, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8) -> u8,
    {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // base zero-page address
            }
            1 => {
                self.vector = add_without_carry(bus.data as u16, reg);
                bus.read(bus.data as u16); // dummy read
            }
            2 => {
                bus.read(self.vector); // operand
            }
            3 => {
                bus.write(bus.addr, bus.data); // dummy write
            }
            4 => {
                bus.write(bus.addr, op(self, bus.data));
                self.poll_intg(ctrl);
            }
            5 => {
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn abi_alu<F>(&mut self, reg: u8, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8),
    {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // low byte of address
            }
            1 => {
                self.vector = bus.data as u16;
                self.read_pc(bus); // high byte of address
            }
            2 => {
                self.vector = (bus.data as u16).wrapping_shl(8) | self.vector;
                let addr = self.vector.wrapping_add(reg as u16);
                if addr & 0xff00 == self.vector & 0xff00 {
                    bus.read(add_without_carry(self.vector, reg)); // operand
                    self.op_cycle += 1;
                } else {
                    bus.read(add_without_carry(self.vector, reg)); // dummy read
                }
            }
            3 => {
                bus.read(self.vector.wrapping_add(reg as u16)); // operand
                self.poll_intg(ctrl);
            }
            4 => {
                op(self, bus.data);
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn abi_store(&mut self, reg: u8, src: u8, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // low byte of address
            }
            1 => {
                self.vector = bus.data as u16;
                self.read_pc(bus); // high byte of address
            }
            2 => {
                self.vector = (bus.data as u16).wrapping_shl(8) | self.vector;
                bus.read(add_without_carry(self.vector, reg)); // dummy read
            }
            3 => {
                bus.write(self.vector.wrapping_add(reg as u16), src);
                self.poll_intg(ctrl);
            }
            4 => {
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn abi_rmw<F>(&mut self, reg: u8, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8) -> u8,
    {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // low byte of address
            }
            1 => {
                self.vector = bus.data as u16;
                self.read_pc(bus); // high byte of address
            }
            2 => {
                self.vector = (bus.data as u16).wrapping_shl(8) | self.vector;
                bus.read(add_without_carry(self.vector, reg)); // dummy read
            }
            3 => {
                bus.read(self.vector.wrapping_add(reg as u16)); // operand
            }
            4 => {
                bus.write(bus.addr, bus.data); // dummy write
            }
            5 => {
                bus.write(bus.addr, op(self, bus.data));
                self.poll_intg(ctrl);
            }
            6 => {
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn izx_alu<F>(&mut self, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8),
    {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // base zero-page pointer to address
            }
            1 => {
                self.vector = add_without_carry(bus.data as u16, self.x);
                bus.read(bus.data as u16); // dummy read
            }
            2 => {
                bus.read(self.vector); // low byte of address
            }
            3 => {
                bus.read(add_without_carry(self.vector, 1)); // high byte of address
                self.vector = bus.data as u16;
            }
            4 => {
                bus.read((bus.data as u16).wrapping_shl(8) | self.vector); // operand
                self.poll_intg(ctrl);
            }
            5 => {
                op(self, bus.data);
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn izx_store(&mut self, src: u8, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // base zero-page pointer to address
            }
            1 => {
                self.vector = add_without_carry(bus.data as u16, self.x);
                bus.read(bus.data as u16); // dummy read
            }
            2 => {
                bus.read(self.vector); // low byte of address
            }
            3 => {
                bus.read(add_without_carry(self.vector, 1)); // high byte of address
                self.vector = bus.data as u16;
            }
            4 => {
                bus.write((bus.data as u16).wrapping_shl(8) | self.vector, src); // operand
                self.poll_intg(ctrl);
            }
            5 => {
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn izx_rmw<F>(&mut self, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8) -> u8,
    {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // base zero-page pointer to address
            }
            1 => {
                self.vector = add_without_carry(bus.data as u16, self.x);
                bus.read(bus.data as u16); // dummy read
            }
            2 => {
                bus.read(self.vector); // low byte of address
            }
            3 => {
                bus.read(add_without_carry(self.vector, 1)); // high byte of address
                self.vector = bus.data as u16;
            }
            4 => {
                bus.read((bus.data as u16).wrapping_shl(8) | self.vector); // operand
            }
            5 => {
                bus.write(bus.addr, bus.data); // dummy write
            }
            6 => {
                bus.write(bus.addr, op(self, bus.data));
                self.poll_intg(ctrl);
            }
            7 => {
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn izy_alu<F>(&mut self, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8),
    {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // zero-page pointer to base address
            }
            1 => {
                self.vector = bus.data as u16;
                bus.read(self.vector); // low byte of base address
            }
            2 => {
                bus.read(add_without_carry(self.vector, 1)); // high byte of base address
                self.vector = bus.data as u16;
            }
            3 => {
                self.vector = (bus.data as u16).wrapping_shl(8) | self.vector;
                let addr = self.vector.wrapping_add(self.y as u16);
                if addr & 0xff00 == self.vector & 0xff00 {
                    bus.read(add_without_carry(self.vector, self.y)); // operand
                    self.op_cycle += 1;
                } else {
                    bus.read(add_without_carry(self.vector, self.y)); // dummy read
                }
            }
            4 => {
                bus.read(self.vector.wrapping_add(self.y as u16)); // operand
                self.poll_intg(ctrl);
            }
            5 => {
                op(self, bus.data);
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn izy_store(&mut self, src: u8, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // zero-page pointer to base address
            }
            1 => {
                self.vector = bus.data as u16;
                bus.read(self.vector); // low byte of base address
            }
            2 => {
                bus.read(add_without_carry(self.vector, 1)); // high byte of base address
                self.vector = bus.data as u16;
            }
            3 => {
                self.vector = (bus.data as u16).wrapping_shl(8) | self.vector;
                bus.read(add_without_carry(self.vector, self.y)); // dummy read
            }
            4 => {
                bus.write(self.vector.wrapping_add(self.y as u16), src); // operand
                self.poll_intg(ctrl);
            }
            5 => {
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn izy_rmw<F>(&mut self, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8) -> u8,
    {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // zero-page pointer to base address
            }
            1 => {
                self.vector = bus.data as u16;
                bus.read(self.vector); // low byte of base address
            }
            2 => {
                bus.read(add_without_carry(self.vector, 1)); // high byte of base address
                self.vector = bus.data as u16;
            }
            3 => {
                self.vector = (bus.data as u16).wrapping_shl(8) | self.vector;
                bus.read(add_without_carry(self.vector, self.y)); // dummy read
            }
            4 => {
                bus.read(self.vector.wrapping_add(self.y as u16)); // operand
            }
            5 => {
                bus.write(bus.addr, bus.data); // dummy write
            }
            6 => {
                bus.write(bus.addr, op(self, bus.data));
                self.poll_intg(ctrl);
            }
            7 => {
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn branch(&mut self, bus: &mut Bus, ctrl: &mut Control, take: bool) {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // offset
                self.poll_intg(ctrl);
            }
            1 => {
                self.vector = self.pc.wrapping_add((bus.data as i8) as u16);
                bus.read(self.pc); // dummy read
                if !take {
                    self.fetch(bus, ctrl);
                }
            }
            2 => {
                bus.read(self.pc & 0xff00 | self.vector & 0xff); // dummy read
                if self.pc & 0xff00 == self.vector & 0xff00 {
                    self.pc = self.vector;
                    self.fetch(bus, ctrl);
                } else {
                    self.poll_intg(ctrl);
                }
            }
            3 => {
                self.pc = self.vector;
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn kil(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                bus.read(self.pc);
            }
            1 => {
                bus.read(0xffff);
            }
            2 | 3 => {
                bus.read(0xfffe);
            }
            _ => {
                bus.read(0xffff);
                if ctrl.res {
                    ctrl.sync = true;
                }
            }
        }
    }
}

// ALU instructions.
impl CPU {
    fn nop(&mut self, _: u8) {}

    fn lda(&mut self, data: u8) {
        self.a = data;
        self.flags.set_nz(self.a);
    }

    fn and(&mut self, data: u8) {
        self.a &= data;
        self.flags.set_nz(self.a);
    }

    fn ora(&mut self, data: u8) {
        self.a |= data;
        self.flags.set_nz(self.a);
    }

    fn eor(&mut self, data: u8) {
        self.a ^= data;
        self.flags.set_nz(self.a);
    }

    fn bit(&mut self, data: u8) {
        self.flags.set_z(self.a & data);
        self.flags.set_from(data, 0b11000000);
    }

    fn anc(&mut self, data: u8) {
        self.and(data);
        self.flags.update_c(self.a & 0x80 != 0);
    }

    fn adc(&mut self, data: u8) {
        if self.flags.d() {
            self.adc_dec(data);
        } else {
            self.adc_hex(data);
        }
    }

    fn adc_dec(&mut self, data: u8) {
        let val = data as u16;
        let a = self.a as u16;
        let mut sl = (a & 0x0f)
            .wrapping_add(val & 0x0f)
            .wrapping_add(self.flags.c() as u16);
        if sl >= 0x0a {
            sl = (sl.wrapping_add(0x06) & 0x0f).wrapping_add(0x10);
        }
        let mut sum = (a & 0xf0).wrapping_add(val & 0xf0).wrapping_add(sl);
        let v = !(a ^ val) & (val ^ sum) & 0x80;
        self.flags.update_v(v != 0);
        self.flags.set_n(sum as u8);

        self.flags
            .set_z(a.wrapping_add(val).wrapping_add(self.flags.c() as u16) as u8);

        if sum >= 0xa0 {
            sum = sum.wrapping_add(0x60);
        }
        self.flags.update_c(sum >= 0x100);
        self.a = sum as u8;
    }

    fn adc_hex(&mut self, data: u8) {
        let val = data as u16;
        let a = self.a as u16;
        let sum = a.wrapping_add(val).wrapping_add(self.flags.c() as u16);
        let v = !(a ^ val) & (val ^ sum) & 0x80;
        self.flags.update_v(v != 0);
        self.flags.update_c(sum > 0xff);
        self.a = sum as u8;
        self.flags.set_nz(self.a);
    }

    fn arr(&mut self, data: u8) {
        self.a &= data;
        let arr = (self.a >> 1) | ((self.flags.c() as u8).wrapping_shl(7));
        self.flags.set_nz(arr);

        if self.flags.d() {
            self.arr_dec(arr);
        } else {
            self.arr_hex(arr);
        }
    }

    fn arr_dec(&mut self, data: u8) {
        let mut val = data;
        self.flags.update_v((val ^ self.a) & 0x40 != 0);
        if (self.a & 0x0f) >= 0x05 {
            val = (val.wrapping_add(0x06) & 0x0f) | (val & 0xf0);
        }
        self.flags.update_c((self.a & 0xf0) >= 0x50);
        if self.flags.c() {
            val = val.wrapping_add(0x60);
        }
        self.a = val;
    }

    fn arr_hex(&mut self, data: u8) {
        self.flags.update_c((data & 0x40) != 0);
        self.flags
            .update_v((data & 0x40) ^ ((data & 0x20) << 1) != 0);
        self.a = data;
    }

    fn asl(&mut self, data: u8) -> u8 {
        let val = data.wrapping_shl(1);
        self.flags.set_nz(val);
        self.flags.update_c(data & 0x80 != 0);
        val
    }

    fn slo(&mut self, data: u8) -> u8 {
        let val = self.asl(data);
        self.ora(val);
        val
    }

    fn rol(&mut self, data: u8) -> u8 {
        let val = data.wrapping_shl(1) | self.flags.c() as u8;
        self.flags.set_nz(val);
        self.flags.update_c(data & 0x80 != 0);
        val
    }

    fn rla(&mut self, data: u8) -> u8 {
        let val = self.rol(data);
        self.and(val);
        val
    }

    fn ror(&mut self, data: u8) -> u8 {
        let val = (data >> 1) | (self.flags.c() as u8).wrapping_shl(7);
        self.flags.set_nz(val);
        self.flags.update_c(data & 0x1 != 0);
        val
    }

    fn rra(&mut self, data: u8) -> u8 {
        let val = self.rol(data);
        self.adc(val);
        val
    }

    fn lsr(&mut self, data: u8) -> u8 {
        let val = data >> 1;
        self.flags.set_nz(val);
        self.flags.update_c(data & 0x01 != 0);
        val
    }

    fn sre(&mut self, data: u8) -> u8 {
        let val = self.lsr(data);
        self.eor(val);
        val
    }

    fn dec(&mut self, data: u8) -> u8 {
        let val = data.wrapping_sub(1);
        self.flags.set_nz(val);
        val
    }
}
