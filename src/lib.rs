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

        ops::exec(self, bus, ctrl);

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

// Addressing modes.
impl CPU {
    fn implicit<F>(op: F, cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self),
    {
        match cpu.op_cycle {
            0 => {
                bus.read(cpu.pc); // dummy read
                cpu.poll_intg(ctrl);
            }
            1 => {
                op(cpu);
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn imm_alu<F>(op: F, cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8),
    {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // operand
                cpu.poll_intg(ctrl);
            }
            1 => {
                op(cpu, bus.data);
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn zp_alu<F>(op: F, cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8),
    {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // zero-page address
            }
            1 => {
                bus.read(bus.data as u16); // operand
                cpu.poll_intg(ctrl);
            }
            2 => {
                op(cpu, bus.data);
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn zp_store(src: u8, cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // zero-page address
            }
            1 => {
                bus.write(bus.data as u16, src);
                cpu.poll_intg(ctrl);
            }
            2 => {
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn zp_rmw<F>(op: F, cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8) -> u8,
    {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // zero-page address
            }
            1 => {
                bus.read(bus.data as u16); // operand
            }
            2 => {
                bus.write(bus.addr, bus.data); // dummy write
            }
            3 => {
                bus.write(bus.addr, op(cpu, bus.data));
                cpu.poll_intg(ctrl);
            }
            4 => {
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn abs_alu<F>(op: F, cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8),
    {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // low byte of address
            }
            1 => {
                cpu.vector = bus.data as u16;
                cpu.read_pc(bus); // high byte of address
            }
            2 => {
                bus.read((bus.data as u16).wrapping_shl(8) | cpu.vector); // operand
                cpu.poll_intg(ctrl);
            }
            3 => {
                op(cpu, bus.data);
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn abs_store(src: u8, cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // low byte of address
            }
            1 => {
                cpu.vector = bus.data as u16;
                cpu.read_pc(bus); // high byte of address
            }
            2 => {
                bus.write((bus.data as u16).wrapping_shl(8) | cpu.vector, src);
                cpu.poll_intg(ctrl);
            }
            3 => {
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn abs_rmw<F>(op: F, cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8) -> u8,
    {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // low byte of address
            }
            1 => {
                cpu.vector = bus.data as u16;
                cpu.read_pc(bus); // high byte of address
            }
            2 => {
                bus.read((bus.data as u16).wrapping_shl(8) | cpu.vector); // operand
            }
            3 => {
                bus.write(bus.addr, bus.data); // dummy write
            }
            4 => {
                bus.write(bus.addr, op(cpu, bus.data));
                cpu.poll_intg(ctrl);
            }
            5 => {
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn zpi_alu<F>(idx: u8, op: F, cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8),
    {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // base zero-page address
            }
            1 => {
                cpu.vector = add_without_carry(bus.data as u16, idx);
                bus.read(bus.data as u16); // dummy read
            }
            2 => {
                bus.read(cpu.vector); // operand
                cpu.poll_intg(ctrl);
            }
            3 => {
                op(cpu, bus.data);
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn zpi_store(idx: u8, src: u8, cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // base zero-page address
            }
            1 => {
                cpu.vector = add_without_carry(bus.data as u16, idx);
                bus.read(bus.data as u16); // dummy read
            }
            2 => {
                bus.write(cpu.vector, src); // operand
                cpu.poll_intg(ctrl);
            }
            3 => {
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn zpi_rmw<F>(idx: u8, op: F, cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8) -> u8,
    {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // base zero-page address
            }
            1 => {
                cpu.vector = add_without_carry(bus.data as u16, idx);
                bus.read(bus.data as u16); // dummy read
            }
            2 => {
                bus.read(cpu.vector); // operand
            }
            3 => {
                bus.write(bus.addr, bus.data); // dummy write
            }
            4 => {
                bus.write(bus.addr, op(cpu, bus.data));
                cpu.poll_intg(ctrl);
            }
            5 => {
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn abi_alu<F>(idx: u8, op: F, cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8),
    {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // low byte of address
            }
            1 => {
                cpu.vector = bus.data as u16;
                cpu.read_pc(bus); // high byte of address
            }
            2 => {
                cpu.vector = (bus.data as u16).wrapping_shl(8) | cpu.vector;
                let addr = cpu.vector.wrapping_add(idx as u16);
                if addr & 0xff00 == cpu.vector & 0xff00 {
                    bus.read(add_without_carry(cpu.vector, idx)); // operand
                    cpu.op_cycle += 1;
                } else {
                    bus.read(add_without_carry(cpu.vector, idx)); // dummy read
                }
            }
            3 => {
                bus.read(cpu.vector.wrapping_add(idx as u16)); // operand
                cpu.poll_intg(ctrl);
            }
            4 => {
                op(cpu, bus.data);
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn abi_store(idx: u8, src: u8, cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // low byte of address
            }
            1 => {
                cpu.vector = bus.data as u16;
                cpu.read_pc(bus); // high byte of address
            }
            2 => {
                cpu.vector = (bus.data as u16).wrapping_shl(8) | cpu.vector;
                bus.read(add_without_carry(cpu.vector, idx)); // dummy read
            }
            3 => {
                bus.write(cpu.vector.wrapping_add(idx as u16), src);
                cpu.poll_intg(ctrl);
            }
            4 => {
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn abi_rmw<F>(idx: u8, op: F, cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8) -> u8,
    {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // low byte of address
            }
            1 => {
                cpu.vector = bus.data as u16;
                cpu.read_pc(bus); // high byte of address
            }
            2 => {
                cpu.vector = (bus.data as u16).wrapping_shl(8) | cpu.vector;
                bus.read(add_without_carry(cpu.vector, idx)); // dummy read
            }
            3 => {
                bus.read(cpu.vector.wrapping_add(idx as u16)); // operand
            }
            4 => {
                bus.write(bus.addr, bus.data); // dummy write
            }
            5 => {
                bus.write(bus.addr, op(cpu, bus.data));
                cpu.poll_intg(ctrl);
            }
            6 => {
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn izx_alu<F>(op: F, cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8),
    {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // base zero-page pointer to address
            }
            1 => {
                cpu.vector = add_without_carry(bus.data as u16, cpu.x);
                bus.read(bus.data as u16); // dummy read
            }
            2 => {
                bus.read(cpu.vector); // low byte of address
            }
            3 => {
                bus.read(add_without_carry(cpu.vector, 1)); // high byte of address
                cpu.vector = bus.data as u16;
            }
            4 => {
                bus.read((bus.data as u16).wrapping_shl(8) | cpu.vector); // operand
                cpu.poll_intg(ctrl);
            }
            5 => {
                op(cpu, bus.data);
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn izx_store(src: u8, cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // base zero-page pointer to address
            }
            1 => {
                cpu.vector = add_without_carry(bus.data as u16, cpu.x);
                bus.read(bus.data as u16); // dummy read
            }
            2 => {
                bus.read(cpu.vector); // low byte of address
            }
            3 => {
                bus.read(add_without_carry(cpu.vector, 1)); // high byte of address
                cpu.vector = bus.data as u16;
            }
            4 => {
                bus.write((bus.data as u16).wrapping_shl(8) | cpu.vector, src); // operand
                cpu.poll_intg(ctrl);
            }
            5 => {
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn izx_rmw<F>(op: F, cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8) -> u8,
    {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // base zero-page pointer to address
            }
            1 => {
                cpu.vector = add_without_carry(bus.data as u16, cpu.x);
                bus.read(bus.data as u16); // dummy read
            }
            2 => {
                bus.read(cpu.vector); // low byte of address
            }
            3 => {
                bus.read(add_without_carry(cpu.vector, 1)); // high byte of address
                cpu.vector = bus.data as u16;
            }
            4 => {
                bus.read((bus.data as u16).wrapping_shl(8) | cpu.vector); // operand
            }
            5 => {
                bus.write(bus.addr, bus.data); // dummy write
            }
            6 => {
                bus.write(bus.addr, op(cpu, bus.data));
                cpu.poll_intg(ctrl);
            }
            7 => {
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn izy_alu<F>(op: F, cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8),
    {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // zero-page pointer to base address
            }
            1 => {
                cpu.vector = bus.data as u16;
                bus.read(cpu.vector); // low byte of base address
            }
            2 => {
                bus.read(add_without_carry(cpu.vector, 1)); // high byte of base address
                cpu.vector = bus.data as u16;
            }
            3 => {
                cpu.vector = (bus.data as u16).wrapping_shl(8) | cpu.vector;
                let addr = cpu.vector.wrapping_add(cpu.y as u16);
                if addr & 0xff00 == cpu.vector & 0xff00 {
                    bus.read(add_without_carry(cpu.vector, cpu.y)); // operand
                    cpu.op_cycle += 1;
                } else {
                    bus.read(add_without_carry(cpu.vector, cpu.y)); // dummy read
                }
            }
            4 => {
                bus.read(cpu.vector.wrapping_add(cpu.y as u16)); // operand
                cpu.poll_intg(ctrl);
            }
            5 => {
                op(cpu, bus.data);
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn izy_store(src: u8, cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // zero-page pointer to base address
            }
            1 => {
                cpu.vector = bus.data as u16;
                bus.read(cpu.vector); // low byte of base address
            }
            2 => {
                bus.read(add_without_carry(cpu.vector, 1)); // high byte of base address
                cpu.vector = bus.data as u16;
            }
            3 => {
                cpu.vector = (bus.data as u16).wrapping_shl(8) | cpu.vector;
                bus.read(add_without_carry(cpu.vector, cpu.y)); // dummy read
            }
            4 => {
                bus.write(cpu.vector.wrapping_add(cpu.y as u16), src); // operand
                cpu.poll_intg(ctrl);
            }
            5 => {
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn izy_rmw<F>(op: F, cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8) -> u8,
    {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // zero-page pointer to base address
            }
            1 => {
                cpu.vector = bus.data as u16;
                bus.read(cpu.vector); // low byte of base address
            }
            2 => {
                bus.read(add_without_carry(cpu.vector, 1)); // high byte of base address
                cpu.vector = bus.data as u16;
            }
            3 => {
                cpu.vector = (bus.data as u16).wrapping_shl(8) | cpu.vector;
                bus.read(add_without_carry(cpu.vector, cpu.y)); // dummy read
            }
            4 => {
                bus.read(cpu.vector.wrapping_add(cpu.y as u16)); // operand
            }
            5 => {
                bus.write(bus.addr, bus.data); // dummy write
            }
            6 => {
                bus.write(bus.addr, op(cpu, bus.data));
                cpu.poll_intg(ctrl);
            }
            7 => {
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }
}

// ALU operations.
impl CPU {
    fn nop(_: &mut Self, _: u8) {}

    fn lda(cpu: &mut Self, data: u8) {
        cpu.a = data;
        cpu.flags.set_nz(cpu.a);
    }

    fn ldy(cpu: &mut Self, data: u8) {
        cpu.y = data;
        cpu.flags.set_nz(cpu.y);
    }

    fn and(cpu: &mut Self, data: u8) {
        cpu.a &= data;
        cpu.flags.set_nz(cpu.a);
    }

    fn ora(cpu: &mut Self, data: u8) {
        cpu.a |= data;
        cpu.flags.set_nz(cpu.a);
    }

    fn eor(cpu: &mut Self, data: u8) {
        cpu.a ^= data;
        cpu.flags.set_nz(cpu.a);
    }

    fn bit(cpu: &mut Self, data: u8) {
        cpu.flags.set_z(cpu.a & data);
        cpu.flags.set_from(data, 0b11000000);
    }

    fn anc(cpu: &mut Self, data: u8) {
        CPU::and(cpu, data);
        cpu.flags.update_c(cpu.a & 0x80 != 0);
    }

    fn adc(cpu: &mut Self, data: u8) {
        if cpu.flags.d() {
            CPU::adc_dec(cpu, data);
        } else {
            CPU::adc_hex(cpu, data);
        }
    }

    fn adc_dec(cpu: &mut Self, data: u8) {
        let val = data as u16;
        let a = cpu.a as u16;
        let mut sl = (a & 0x0f)
            .wrapping_add(val & 0x0f)
            .wrapping_add(cpu.flags.c() as u16);
        if sl >= 0x0a {
            sl = (sl.wrapping_add(0x06) & 0x0f).wrapping_add(0x10);
        }
        let mut sum = (a & 0xf0).wrapping_add(val & 0xf0).wrapping_add(sl);
        let v = !(a ^ val) & (val ^ sum) & 0x80;
        cpu.flags.update_v(v != 0);
        cpu.flags.set_n(sum as u8);

        cpu.flags
            .set_z(a.wrapping_add(val).wrapping_add(cpu.flags.c() as u16) as u8);

        if sum >= 0xa0 {
            sum = sum.wrapping_add(0x60);
        }
        cpu.flags.update_c(sum >= 0x100);
        cpu.a = sum as u8;
    }

    fn adc_hex(cpu: &mut Self, data: u8) {
        let val = data as u16;
        let a = cpu.a as u16;
        let sum = a.wrapping_add(val).wrapping_add(cpu.flags.c() as u16);
        let v = !(a ^ val) & (val ^ sum) & 0x80;
        cpu.flags.update_v(v != 0);
        cpu.flags.update_c(sum > 0xff);
        cpu.a = sum as u8;
        cpu.flags.set_nz(cpu.a);
    }

    fn arr(cpu: &mut Self, data: u8) {
        cpu.a &= data;
        let arr = (cpu.a >> 1) | ((cpu.flags.c() as u8).wrapping_shl(7));
        cpu.flags.set_nz(arr);

        if cpu.flags.d() {
            CPU::arr_dec(cpu, arr);
        } else {
            CPU::arr_hex(cpu, arr);
        }
    }

    fn arr_dec(cpu: &mut Self, data: u8) {
        let mut val = data;
        cpu.flags.update_v((val ^ cpu.a) & 0x40 != 0);
        if (cpu.a & 0x0f) >= 0x05 {
            val = (val.wrapping_add(0x06) & 0x0f) | (val & 0xf0);
        }
        cpu.flags.update_c((cpu.a & 0xf0) >= 0x50);
        if cpu.flags.c() {
            val = val.wrapping_add(0x60);
        }
        cpu.a = val;
    }

    fn arr_hex(cpu: &mut Self, data: u8) {
        cpu.flags.update_c((data & 0x40) != 0);
        cpu.flags
            .update_v((data & 0x40) ^ ((data & 0x20) << 1) != 0);
        cpu.a = data;
    }

    fn xaa(cpu: &mut Self, data: u8) {
        CPU::lda(cpu, (cpu.a | 0xef) & cpu.x & data);
    }

    fn asl(cpu: &mut Self, data: u8) -> u8 {
        let val = data.wrapping_shl(1);
        cpu.flags.set_nz(val);
        cpu.flags.update_c(data & 0x80 != 0);
        val
    }

    fn slo(cpu: &mut Self, data: u8) -> u8 {
        let val = CPU::asl(cpu, data);
        CPU::ora(cpu, val);
        val
    }

    fn rol(cpu: &mut Self, data: u8) -> u8 {
        let val = data.wrapping_shl(1) | cpu.flags.c() as u8;
        cpu.flags.set_nz(val);
        cpu.flags.update_c(data & 0x80 != 0);
        val
    }

    fn rla(cpu: &mut Self, data: u8) -> u8 {
        let val = CPU::rol(cpu, data);
        CPU::and(cpu, val);
        val
    }

    fn ror(cpu: &mut Self, data: u8) -> u8 {
        let val = (data >> 1) | (cpu.flags.c() as u8).wrapping_shl(7);
        cpu.flags.set_nz(val);
        cpu.flags.update_c(data & 0x1 != 0);
        val
    }

    fn rra(cpu: &mut Self, data: u8) -> u8 {
        let val = CPU::rol(cpu, data);
        CPU::adc(cpu, val);
        val
    }

    fn lsr(cpu: &mut Self, data: u8) -> u8 {
        let val = data >> 1;
        cpu.flags.set_nz(val);
        cpu.flags.update_c(data & 0x01 != 0);
        val
    }

    fn sre(cpu: &mut Self, data: u8) -> u8 {
        let val = CPU::lsr(cpu, data);
        CPU::eor(cpu, val);
        val
    }

    fn dec(cpu: &mut Self, data: u8) -> u8 {
        let val = data.wrapping_sub(1);
        cpu.flags.set_nz(val);
        val
    }
}

// Control flow.
impl CPU {
    fn branch(take: bool, cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // offset
                cpu.poll_intg(ctrl);
            }
            1 => {
                cpu.vector = cpu.pc.wrapping_add((bus.data as i8) as u16);
                bus.read(cpu.pc); // dummy read
                if !take {
                    cpu.fetch(bus, ctrl);
                }
            }
            2 => {
                bus.read(cpu.pc & 0xff00 | cpu.vector & 0xff); // dummy read
                if cpu.pc & 0xff00 == cpu.vector & 0xff00 {
                    cpu.pc = cpu.vector;
                    cpu.fetch(bus, ctrl);
                } else {
                    cpu.poll_intg(ctrl);
                }
            }
            3 => {
                cpu.pc = cpu.vector;
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn kil(cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 => {
                bus.read(cpu.pc);
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

    fn brk(cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 => {
                bus.read(cpu.pc); // dummy read
            }
            1 => {
                if ctrl.res {
                    cpu.read_sp(bus); // dummy read
                    cpu.sp = cpu.sp.wrapping_sub(1);
                } else {
                    cpu.write_sp(bus, (cpu.pc >> 8) as u8);
                }
            }
            2 => {
                if ctrl.res {
                    cpu.read_sp(bus); // dummy read
                    cpu.sp = cpu.sp.wrapping_sub(1);
                } else {
                    cpu.write_sp(bus, cpu.pc as u8);
                }
            }
            3 => {
                // ??? is this here or at end of prev cycle?
                if ctrl.res {
                    cpu.vector = 0xfffc;
                } else if cpu.nmi_pending || ctrl.nmi {
                    cpu.vector = 0xfffa;
                } else {
                    cpu.vector = 0xfffe;
                }

                if ctrl.res {
                    cpu.read_sp(bus); // dummy read
                    cpu.sp = cpu.sp.wrapping_sub(1);
                } else if cpu.intg {
                    cpu.write_sp(bus, cpu.flags.0 & 0b1110_1111);
                } else {
                    cpu.write_sp(bus, cpu.flags.0 | 0b0011_0000);
                }
            }
            4 => {
                bus.read(cpu.vector); // low byte of target

                // ??? is this here or at start of next cycle?
                cpu.flags.update_i(true);
                ctrl.res = false;
                cpu.intg = false;
            }
            5 => {
                let vec_lo = bus.data as u16;
                bus.read(cpu.vector.wrapping_add(1)); // high byte of target
                cpu.vector = vec_lo;
            }
            6 => {
                cpu.pc = (bus.data as u16).wrapping_shl(8) | cpu.vector;
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn rti(cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 => {
                bus.read(cpu.pc); // dummy read
            }
            1 => {
                cpu.read_sp(bus); // dummy read
                cpu.sp = cpu.sp.wrapping_add(1);
            }
            2 => {
                cpu.read_sp(bus);
                cpu.sp = cpu.sp.wrapping_add(1);
            }
            3 => {
                cpu.flags.set_from(bus.data, 0b11111111);
                cpu.read_sp(bus); // low byte of return address
                cpu.sp = cpu.sp.wrapping_add(1);
            }
            4 => {
                cpu.vector = bus.data as u16;
                cpu.read_sp(bus); // high byte of return address
                cpu.poll_intg(ctrl);
            }
            5 => {
                cpu.pc = (bus.data as u16).wrapping_shl(8) | cpu.vector;
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn jmp(cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // low byte of target address
            }
            1 => {
                cpu.vector = bus.data as u16;
                cpu.read_pc(bus); // high byte of target address
                cpu.poll_intg(ctrl);
            }
            2 => {
                cpu.pc = (bus.data as u16).wrapping_shl(8) | cpu.vector;
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn jmp_ind(cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // low byte of pointer address
            }
            1 => {
                cpu.vector = bus.data as u16;
                cpu.read_pc(bus); // high byte of pointer address
            }
            2 => {
                cpu.vector = (bus.data as u16).wrapping_shl(8) | cpu.vector;
                bus.read(cpu.vector); // low byte of target address
            }
            3 => {
                bus.read(add_without_carry(cpu.vector, 1)); // high byte of target address
                cpu.vector = bus.data as u16;
                cpu.poll_intg(ctrl);
            }
            4 => {
                cpu.pc = (bus.data as u16).wrapping_shl(8) | cpu.vector;
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn jsr(cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 => {
                cpu.read_pc(bus); // low byte of target
            }
            1 => {
                cpu.vector = bus.data as u16;
                cpu.read_sp(bus); // dummy read
            }
            2 => {
                cpu.write_sp(bus, (cpu.pc >> 8) as u8);
            }
            3 => {
                cpu.write_sp(bus, cpu.pc as u8);
            }
            4 => {
                bus.read(cpu.pc); // high byte of target
                cpu.poll_intg(ctrl);
            }
            5 => {
                cpu.pc = (bus.data as u16).wrapping_shl(8) | cpu.vector;
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn rts(cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 => {
                bus.read(cpu.pc); // dummy read
            }
            1 => {
                cpu.read_sp(bus); // dummy read
                cpu.sp = cpu.sp.wrapping_add(1);
            }
            2 => {
                cpu.read_sp(bus); // low byte of return address
                cpu.sp = cpu.sp.wrapping_add(1);
            }
            3 => {
                cpu.vector = bus.data as u16;
                cpu.read_sp(bus); // high byte of return address
            }
            4 => {
                cpu.pc = (bus.data as u16).wrapping_shl(8) | cpu.vector;
                cpu.read_pc(bus); // dummy read
                cpu.poll_intg(ctrl);
            }
            5 => {
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }
}

// Stack operations.
impl CPU {
    fn pha(cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 => {
                bus.read(cpu.pc); // dummy read
            }
            1 => {
                cpu.write_sp(bus, cpu.a);
                cpu.poll_intg(ctrl);
            }
            2 => {
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn pla(cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 => {
                bus.read(cpu.pc); // dummy read
            }
            1 => {
                cpu.read_sp(bus); // dummy read
                cpu.sp = cpu.sp.wrapping_add(1);
            }
            2 => {
                cpu.read_sp(bus); // data
                cpu.poll_intg(ctrl);
            }
            3 => {
                cpu.a = bus.data;
                cpu.flags.set_nz(cpu.a);
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn php(cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 => {
                bus.read(cpu.pc); // dummy read
            }
            1 => {
                cpu.write_sp(bus, cpu.flags.0 | 0b0011_0000);
                cpu.poll_intg(ctrl);
            }
            2 => {
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn plp(cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 => {
                bus.read(cpu.pc); // dummy read
            }
            1 => {
                cpu.read_sp(bus); // dummy read
                cpu.sp = cpu.sp.wrapping_add(1);
            }
            2 => {
                cpu.read_sp(bus); // data
                cpu.poll_intg(ctrl);
            }
            3 => {
                cpu.flags.set_from(bus.data, 0b11111111);
                cpu.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }
}

// Unstable address high-byte opcodes.
impl CPU {
    fn ahx_izy(cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 | 1 | 2 | 3 | 5 => {
                CPU::izy_store(0, cpu, bus, ctrl);
            }
            4 => {
                CPU::unstable_high_byte(|c| c.a & c.x, cpu.y, cpu, bus);
                cpu.poll_intg(ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn tas(cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 | 1 | 2 | 4 => {
                CPU::abi_store(cpu.y, 0, cpu, bus, ctrl);
            }
            3 => {
                CPU::unstable_high_byte(
                    |c| {
                        c.sp = c.a & c.x;
                        c.sp
                    },
                    cpu.y,
                    cpu,
                    bus,
                );
                cpu.poll_intg(ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn shy(cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 | 1 | 2 | 4 => {
                CPU::abi_store(cpu.x, 0, cpu, bus, ctrl);
            }
            3 => {
                CPU::unstable_high_byte(|c| c.y, cpu.x, cpu, bus);
                cpu.poll_intg(ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn shx(cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 | 1 | 2 | 4 => {
                CPU::abi_store(cpu.y, 0, cpu, bus, ctrl);
            }
            3 => {
                CPU::unstable_high_byte(|c| c.x, cpu.y, cpu, bus);
                cpu.poll_intg(ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn ahx_abi(cpu: &mut Self, bus: &mut Bus, ctrl: &mut Control) {
        match cpu.op_cycle {
            0 | 1 | 2 | 4 => {
                CPU::abi_store(cpu.y, 0, cpu, bus, ctrl);
            }
            3 => {
                CPU::unstable_high_byte(|c| c.a, cpu.y, cpu, bus);
                cpu.poll_intg(ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn unstable_high_byte<F>(data: F, idx: u8, cpu: &mut Self, bus: &mut Bus)
    where
        F: Fn(&mut Self) -> u8,
    {
        // TODO: if rdy, don't & with hi+1?
        let val = data(cpu) & ((cpu.vector >> 8) as u8).wrapping_add(1);
        if bus.addr != cpu.vector.wrapping_add(idx as u16) {
            bus.write((val as u16) << 8 | bus.addr & 0xff, val);
        } else {
            bus.write(bus.addr, val);
        }
    }
}
