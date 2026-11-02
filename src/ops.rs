use super::*;

fn add_without_carry(a: u16, b: u8) -> u16 {
    a.wrapping_add(b as u16) & 0xff00
}

impl CPU {
    // BRK
    fn op_00(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                bus.read(self.pc); // dummy read
            }
            1 => {
                if ctrl.res {
                    bus.read(self.sp as u16 | 0x100); // dummy read
                    self.sp = self.sp.wrapping_sub(1);
                } else {
                    self.write_sp(bus, (self.pc >> 8) as u8);
                }
            }
            2 => {
                if ctrl.res {
                    bus.read(self.sp as u16 | 0x100); // dummy read
                    self.sp = self.sp.wrapping_sub(1);
                } else {
                    self.write_sp(bus, self.pc as u8);
                }
            }
            3 => {
                // ??? is this here or at end of prev cycle?
                if ctrl.res {
                    self.vector = 0xfffc;
                } else if self.nmi_pending || ctrl.nmi {
                    self.vector = 0xfffa;
                } else {
                    self.vector = 0xfffe;
                }

                if ctrl.res {
                    bus.read(self.sp as u16 | 0x100); // dummy read
                    self.sp = self.sp.wrapping_sub(1);
                } else if self.intg {
                    self.write_sp(bus, self.flags.0 & 0b1110_1111);
                } else {
                    self.write_sp(bus, self.flags.0 | 0b0011_0000);
                }
            }
            4 => {
                bus.read(self.vector); // low byte of target

                // ??? is this here or at start of next cycle?
                self.flags.update_i(true);
                ctrl.res = false;
                self.intg = false;
            }
            5 => {
                let vec_lo = bus.data as u16;
                bus.read(self.vector.wrapping_add(1)); // high byte of target
                self.vector = vec_lo;
            }
            6 => {
                self.pc = (bus.data as u16).wrapping_shl(8) | self.vector;
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    // ORA ($nn,X)
    fn op_01(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 5 {
            self.izx(bus);
            if self.op_cycle == 4 {
                self.poll_intg(ctrl);
            }
        } else {
            self.ora(bus.data);
            self.fetch(bus, ctrl);
        }
    }

    // KIL
    fn op_02(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.kil(bus, ctrl);
    }

    // SLO ($nn, X)
    fn op_03(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 5 {
            self.izx(bus);
        } else {
            match self.op_cycle {
                5 => {
                    bus.write(bus.addr, bus.data); // dummy write
                }
                6 => {
                    let data = self.asl(bus.data);
                    bus.write(bus.addr, data);
                    self.ora(data);
                    self.poll_intg(ctrl);
                }
                7 => {
                    self.fetch(bus, ctrl);
                }
                _ => unreachable!(),
            }
        }
    }

    // NOP $nn
    fn op_04(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 2 {
            self.zp(bus);
            if self.op_cycle == 1 {
                self.poll_intg(ctrl);
            }
        } else {
            self.fetch(bus, ctrl);
        }
    }

    // ORA $nn
    fn op_05(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 2 {
            self.zp(bus);
            if self.op_cycle == 1 {
                self.poll_intg(ctrl);
            }
        } else {
            self.ora(bus.data);
            self.fetch(bus, ctrl);
        }
    }

    // ASL $nn
    fn op_06(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 2 {
            self.zp(bus);
        } else {
            match self.op_cycle {
                2 => {
                    bus.write(bus.addr, bus.data); // dummy write
                }
                3 => {
                    bus.write(bus.addr, self.asl(bus.data));
                    self.poll_intg(ctrl);
                }
                4 => {
                    self.fetch(bus, ctrl);
                }
                _ => unreachable!(),
            }
        }
    }

    // SLO $nn
    fn op_07(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 2 {
            self.zp(bus);
        } else {
            match self.op_cycle {
                2 => {
                    bus.write(bus.addr, bus.data); // dummy write
                }
                3 => {
                    let data = self.asl(bus.data);
                    bus.write(bus.addr, data);
                    self.ora(data);
                    self.poll_intg(ctrl);
                }
                4 => {
                    self.fetch(bus, ctrl);
                }
                _ => unreachable!(),
            }
        }
    }

    // PHP
    fn op_08(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                bus.read(self.pc); // dummy read
            }
            1 => {
                self.write_sp(bus, self.flags.0 | 0b0011_0000);
                self.poll_intg(ctrl);
            }
            2 => {
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    // ORA #nn
    fn op_09(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // operand
                self.poll_intg(ctrl);
            }
            1 => {
                self.ora(bus.data);
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    // ASLA
    fn op_0a(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                bus.read(self.pc); // dummy read
                self.poll_intg(ctrl);
            }
            1 => {
                self.a = self.asl(self.a);
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    // ANC #nn
    fn op_0b(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // operand
                self.poll_intg(ctrl);
            }
            1 => {
                self.and(bus.data);
                self.flags.update_c(self.a & 0x80 != 0);
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    // NOP $nnnn
    fn op_0c(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 3 {
            self.abs(bus);
            if self.op_cycle == 2 {
                self.poll_intg(ctrl);
            }
        } else {
            self.fetch(bus, ctrl);
        }
    }

    // ORA $nnnn
    fn op_0d(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 3 {
            self.abs(bus);
            if self.op_cycle == 2 {
                self.poll_intg(ctrl);
            }
        } else {
            self.ora(bus.data);
            self.fetch(bus, ctrl);
        }
    }

    // ASL $nnnn
    fn op_0e(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 3 {
            self.abs(bus);
        } else {
            match self.op_cycle {
                3 => {
                    bus.write(bus.addr, bus.data); // dummy write
                }
                4 => {
                    bus.write(bus.addr, self.asl(bus.data));
                    self.poll_intg(ctrl);
                }
                5 => {
                    self.fetch(bus, ctrl);
                }
                _ => unreachable!(),
            }
        }
    }

    // SLO $nnnn
    fn op_0f(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 3 {
            self.abs(bus);
        } else {
            match self.op_cycle {
                3 => {
                    bus.write(bus.addr, bus.data); // dummy write
                }
                4 => {
                    let data = self.asl(bus.data);
                    bus.write(bus.addr, data);
                    self.ora(data);
                    self.poll_intg(ctrl);
                }
                5 => {
                    self.fetch(bus, ctrl);
                }
                _ => unreachable!(),
            }
        }
    }

    // BPL
    fn op_10(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.branch(bus, ctrl, !self.flags.n());
    }

    // ORA ($nn),Y
    fn op_11(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 5 {
            self.izy(bus, false);
            if self.op_cycle == 4 {
                self.poll_intg(ctrl);
            }
        } else {
            self.ora(bus.data);
            self.fetch(bus, ctrl);
        }
    }

    // KIL
    fn op_12(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.kil(bus, ctrl);
    }

    // SLO $(nn), Y
    fn op_13(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 5 {
            self.izy(bus, true);
        } else {
            match self.op_cycle {
                5 => {
                    bus.write(bus.addr, bus.data); // dummy write
                }
                6 => {
                    let data = self.asl(bus.data);
                    bus.write(bus.addr, data);
                    self.ora(data);
                    self.poll_intg(ctrl);
                }
                7 => {
                    self.fetch(bus, ctrl);
                }
                _ => unreachable!(),
            }
        }
    }

    // NOP $nn,X
    fn op_14(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 3 {
            self.zpi(bus);
            if self.op_cycle == 2 {
                self.poll_intg(ctrl);
            }
        } else {
            self.fetch(bus, ctrl);
        }
    }

    // ORA $nn,X
    fn op_15(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 3 {
            self.zpi(bus);
            if self.op_cycle == 2 {
                self.poll_intg(ctrl);
            }
        } else {
            self.ora(bus.data);
            self.fetch(bus, ctrl);
        }
    }

    // ASL $nn,X
    fn op_16(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 3 {
            self.zpi(bus);
        } else {
            match self.op_cycle {
                3 => {
                    bus.write(bus.addr, bus.data); // dummy write
                }
                4 => {
                    bus.write(bus.addr, self.asl(bus.data));
                    self.poll_intg(ctrl);
                }
                5 => {
                    self.fetch(bus, ctrl);
                }
                _ => unreachable!(),
            }
        }
    }

    // SLO $nn,X
    fn op_17(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 3 {
            self.zpi(bus);
        } else {
            match self.op_cycle {
                3 => {
                    bus.write(bus.addr, bus.data); // dummy write
                }
                4 => {
                    let data = self.asl(bus.data);
                    bus.write(bus.addr, data);
                    self.ora(data);
                    self.poll_intg(ctrl);
                }
                5 => {
                    self.fetch(bus, ctrl);
                }
                _ => unreachable!(),
            }
        }
    }

    // CLC
    fn op_18(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                bus.read(self.pc); // dummy read
                self.poll_intg(ctrl);
            }
            1 => {
                self.flags.update_c(false);
            }
            _ => unreachable!(),
        }
    }

    // ORA $nnnn,Y
    fn op_19(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 4 {
            self.abi(bus, self.y, false);
            if self.op_cycle == 3 {
                self.poll_intg(ctrl);
            }
        } else {
            self.ora(bus.data);
            self.fetch(bus, ctrl);
        }
    }

    // NOP
    fn op_1a(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                bus.read(self.pc); // dummy read
                self.poll_intg(ctrl);
            }
            1 => {
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    // SLO $nnnn,Y
    fn op_1b(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 4 {
            self.abi(bus, self.y, true);
        } else {
            match self.op_cycle {
                4 => {
                    bus.write(bus.addr, bus.data); // dummy write
                }
                5 => {
                    let data = self.asl(bus.data);
                    bus.write(bus.addr, data);
                    self.ora(data);
                    self.poll_intg(ctrl);
                }
                6 => {
                    self.fetch(bus, ctrl);
                }
                _ => unreachable!(),
            }
        }
    }

    // NOP $nnnn,X
    fn op_1c(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 4 {
            self.abi(bus, self.x, false);
            if self.op_cycle == 3 {
                self.poll_intg(ctrl);
            }
        } else {
            self.fetch(bus, ctrl);
        }
    }

    // ORA $nnnn,X
    fn op_1d(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 4 {
            self.abi(bus, self.x, false);
            if self.op_cycle == 3 {
                self.poll_intg(ctrl);
            }
        } else {
            self.ora(bus.data);
            self.fetch(bus, ctrl);
        }
    }

    // ASL $nnnn,X
    fn op_1e(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 4 {
            self.abi(bus, self.x, true);
        } else {
            match self.op_cycle {
                4 => {
                    bus.write(bus.addr, bus.data); // dummy write
                }
                5 => {
                    bus.write(bus.addr, self.asl(bus.data));
                    self.poll_intg(ctrl);
                }
                6 => {
                    self.fetch(bus, ctrl);
                }
                _ => unreachable!(),
            }
        }
    }

    // SLO $nnnn,X
    fn op_1f(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        if self.op_cycle < 4 {
            self.abi(bus, self.x, true);
        } else {
            match self.op_cycle {
                4 => {
                    bus.write(bus.addr, bus.data); // dummy write
                }
                5 => {
                    let data = self.asl(bus.data);
                    bus.write(bus.addr, data);
                    self.ora(data);
                    self.poll_intg(ctrl);
                }
                6 => {
                    self.fetch(bus, ctrl);
                }
                _ => unreachable!(),
            }
        }
    }

    // JSR
    fn op_20(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // low byte of target
            }
            1 => {
                self.vector = bus.data as u16;
                bus.read(self.sp as u16 | 0x100); // dummy read
            }
            2 => {
                self.write_sp(bus, (self.pc >> 8) as u8);
            }
            3 => {
                self.write_sp(bus, self.pc as u8);
            }
            4 => {
                bus.read(self.pc); // high byte of target
                self.poll_intg(ctrl);
            }
            5 => {
                self.pc = (bus.data as u16).wrapping_shl(8) | self.vector;
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }
    fn op_21(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_22(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_23(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_24(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_25(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_26(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_27(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_28(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_29(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_2a(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_2b(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_2c(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_2d(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_2e(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_2f(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_30(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_31(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_32(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_33(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_34(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_35(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_36(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_37(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_38(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_39(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_3a(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_3b(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_3c(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_3d(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_3e(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_3f(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_40(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_41(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_42(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_43(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_44(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_45(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_46(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_47(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_48(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_49(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_4a(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_4b(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_4c(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_4d(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_4e(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_4f(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_50(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_51(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_52(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_53(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_54(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_55(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_56(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_57(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_58(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_59(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_5a(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_5b(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_5c(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_5d(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_5e(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_5f(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_60(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_61(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_62(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_63(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_64(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_65(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_66(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_67(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_68(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_69(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_6a(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_6b(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_6c(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_6d(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_6e(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_6f(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_70(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_71(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_72(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_73(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_74(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_75(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_76(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_77(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_78(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_79(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_7a(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_7b(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_7c(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_7d(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_7e(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_7f(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_80(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_81(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_82(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_83(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_84(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_85(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_86(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_87(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_88(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_89(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_8a(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_8b(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_8c(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_8d(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_8e(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_8f(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_90(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_91(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_92(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_93(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_94(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_95(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_96(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_97(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_98(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_99(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_9a(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_9b(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_9c(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_9d(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_9e(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_9f(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_a0(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_a1(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_a2(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_a3(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_a4(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_a5(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_a6(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_a7(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_a8(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_a9(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_aa(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_ab(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_ac(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_ad(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_ae(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_af(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_b0(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_b1(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_b2(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_b3(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_b4(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_b5(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_b6(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_b7(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_b8(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_b9(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_ba(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_bb(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_bc(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_bd(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_be(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_bf(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_c0(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_c1(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_c2(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_c3(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_c4(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_c5(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_c6(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_c7(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_c8(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_c9(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_ca(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_cb(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_cc(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_cd(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_ce(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_cf(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_d0(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_d1(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_d2(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_d3(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_d4(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_d5(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_d6(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_d7(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_d8(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_d9(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_da(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_db(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_dc(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_dd(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_de(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_df(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_e0(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_e1(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_e2(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_e3(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_e4(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_e5(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_e6(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_e7(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_e8(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_e9(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_ea(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_eb(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_ec(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_ed(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_ee(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_ef(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_f0(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_f1(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_f2(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_f3(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_f4(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_f5(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_f6(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_f7(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_f8(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_f9(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_fa(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_fb(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_fc(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_fd(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_fe(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }
    fn op_ff(&mut self) {
        match self.op_cycle {
            0 => {}
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            7 => {}
            _ => {
                unreachable!();
            }
        }
    }

    pub(super) fn exec(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        match self.op {
            0x00 => self.op_00(bus, ctrl),
            0x01 => self.op_01(bus, ctrl),
            0x02 => self.op_02(bus, ctrl),
            0x03 => self.op_03(bus, ctrl),
            0x04 => self.op_04(bus, ctrl),
            0x05 => self.op_05(bus, ctrl),
            0x06 => self.op_06(bus, ctrl),
            0x07 => self.op_07(bus, ctrl),
            0x08 => self.op_08(bus, ctrl),
            0x09 => self.op_09(bus, ctrl),
            0x0a => self.op_0a(bus, ctrl),
            0x0b => self.op_0b(bus, ctrl),
            0x0c => self.op_0c(bus, ctrl),
            0x0d => self.op_0d(bus, ctrl),
            0x0e => self.op_0e(bus, ctrl),
            0x0f => self.op_0f(bus, ctrl),
            0x10 => self.op_10(bus, ctrl),
            0x11 => self.op_11(bus, ctrl),
            0x12 => self.op_12(bus, ctrl),
            0x13 => self.op_13(bus, ctrl),
            0x14 => self.op_14(bus, ctrl),
            0x15 => self.op_15(bus, ctrl),
            0x16 => self.op_16(bus, ctrl),
            0x17 => self.op_17(bus, ctrl),
            0x18 => self.op_18(bus, ctrl),
            0x19 => self.op_19(bus, ctrl),
            0x1a => self.op_1a(bus, ctrl),
            0x1b => self.op_1b(bus, ctrl),
            0x1c => self.op_1c(bus, ctrl),
            0x1d => self.op_1d(bus, ctrl),
            0x1e => self.op_1e(bus, ctrl),
            0x1f => self.op_1f(bus, ctrl),
            0x20 => self.op_20(),
            0x21 => self.op_21(),
            0x22 => self.op_22(),
            0x23 => self.op_23(),
            0x24 => self.op_24(),
            0x25 => self.op_25(),
            0x26 => self.op_26(),
            0x27 => self.op_27(),
            0x28 => self.op_28(),
            0x29 => self.op_29(),
            0x2a => self.op_2a(),
            0x2b => self.op_2b(),
            0x2c => self.op_2c(),
            0x2d => self.op_2d(),
            0x2e => self.op_2e(),
            0x2f => self.op_2f(),
            0x30 => self.op_30(),
            0x31 => self.op_31(),
            0x32 => self.op_32(),
            0x33 => self.op_33(),
            0x34 => self.op_34(),
            0x35 => self.op_35(),
            0x36 => self.op_36(),
            0x37 => self.op_37(),
            0x38 => self.op_38(),
            0x39 => self.op_39(),
            0x3a => self.op_3a(),
            0x3b => self.op_3b(),
            0x3c => self.op_3c(),
            0x3d => self.op_3d(),
            0x3e => self.op_3e(),
            0x3f => self.op_3f(),
            0x40 => self.op_40(),
            0x41 => self.op_41(),
            0x42 => self.op_42(),
            0x43 => self.op_43(),
            0x44 => self.op_44(),
            0x45 => self.op_45(),
            0x46 => self.op_46(),
            0x47 => self.op_47(),
            0x48 => self.op_48(),
            0x49 => self.op_49(),
            0x4a => self.op_4a(),
            0x4b => self.op_4b(),
            0x4c => self.op_4c(),
            0x4d => self.op_4d(),
            0x4e => self.op_4e(),
            0x4f => self.op_4f(),
            0x50 => self.op_50(),
            0x51 => self.op_51(),
            0x52 => self.op_52(),
            0x53 => self.op_53(),
            0x54 => self.op_54(),
            0x55 => self.op_55(),
            0x56 => self.op_56(),
            0x57 => self.op_57(),
            0x58 => self.op_58(),
            0x59 => self.op_59(),
            0x5a => self.op_5a(),
            0x5b => self.op_5b(),
            0x5c => self.op_5c(),
            0x5d => self.op_5d(),
            0x5e => self.op_5e(),
            0x5f => self.op_5f(),
            0x60 => self.op_60(),
            0x61 => self.op_61(),
            0x62 => self.op_62(),
            0x63 => self.op_63(),
            0x64 => self.op_64(),
            0x65 => self.op_65(),
            0x66 => self.op_66(),
            0x67 => self.op_67(),
            0x68 => self.op_68(),
            0x69 => self.op_69(),
            0x6a => self.op_6a(),
            0x6b => self.op_6b(),
            0x6c => self.op_6c(),
            0x6d => self.op_6d(),
            0x6e => self.op_6e(),
            0x6f => self.op_6f(),
            0x70 => self.op_70(),
            0x71 => self.op_71(),
            0x72 => self.op_72(),
            0x73 => self.op_73(),
            0x74 => self.op_74(),
            0x75 => self.op_75(),
            0x76 => self.op_76(),
            0x77 => self.op_77(),
            0x78 => self.op_78(),
            0x79 => self.op_79(),
            0x7a => self.op_7a(),
            0x7b => self.op_7b(),
            0x7c => self.op_7c(),
            0x7d => self.op_7d(),
            0x7e => self.op_7e(),
            0x7f => self.op_7f(),
            0x80 => self.op_80(),
            0x81 => self.op_81(),
            0x82 => self.op_82(),
            0x83 => self.op_83(),
            0x84 => self.op_84(),
            0x85 => self.op_85(),
            0x86 => self.op_86(),
            0x87 => self.op_87(),
            0x88 => self.op_88(),
            0x89 => self.op_89(),
            0x8a => self.op_8a(),
            0x8b => self.op_8b(),
            0x8c => self.op_8c(),
            0x8d => self.op_8d(),
            0x8e => self.op_8e(),
            0x8f => self.op_8f(),
            0x90 => self.op_90(),
            0x91 => self.op_91(),
            0x92 => self.op_92(),
            0x93 => self.op_93(),
            0x94 => self.op_94(),
            0x95 => self.op_95(),
            0x96 => self.op_96(),
            0x97 => self.op_97(),
            0x98 => self.op_98(),
            0x99 => self.op_99(),
            0x9a => self.op_9a(),
            0x9b => self.op_9b(),
            0x9c => self.op_9c(),
            0x9d => self.op_9d(),
            0x9e => self.op_9e(),
            0x9f => self.op_9f(),
            0xa0 => self.op_a0(),
            0xa1 => self.op_a1(),
            0xa2 => self.op_a2(),
            0xa3 => self.op_a3(),
            0xa4 => self.op_a4(),
            0xa5 => self.op_a5(),
            0xa6 => self.op_a6(),
            0xa7 => self.op_a7(),
            0xa8 => self.op_a8(),
            0xa9 => self.op_a9(),
            0xaa => self.op_aa(),
            0xab => self.op_ab(),
            0xac => self.op_ac(),
            0xad => self.op_ad(),
            0xae => self.op_ae(),
            0xaf => self.op_af(),
            0xb0 => self.op_b0(),
            0xb1 => self.op_b1(),
            0xb2 => self.op_b2(),
            0xb3 => self.op_b3(),
            0xb4 => self.op_b4(),
            0xb5 => self.op_b5(),
            0xb6 => self.op_b6(),
            0xb7 => self.op_b7(),
            0xb8 => self.op_b8(),
            0xb9 => self.op_b9(),
            0xba => self.op_ba(),
            0xbb => self.op_bb(),
            0xbc => self.op_bc(),
            0xbd => self.op_bd(),
            0xbe => self.op_be(),
            0xbf => self.op_bf(),
            0xc0 => self.op_c0(),
            0xc1 => self.op_c1(),
            0xc2 => self.op_c2(),
            0xc3 => self.op_c3(),
            0xc4 => self.op_c4(),
            0xc5 => self.op_c5(),
            0xc6 => self.op_c6(),
            0xc7 => self.op_c7(),
            0xc8 => self.op_c8(),
            0xc9 => self.op_c9(),
            0xca => self.op_ca(),
            0xcb => self.op_cb(),
            0xcc => self.op_cc(),
            0xcd => self.op_cd(),
            0xce => self.op_ce(),
            0xcf => self.op_cf(),
            0xd0 => self.op_d0(),
            0xd1 => self.op_d1(),
            0xd2 => self.op_d2(),
            0xd3 => self.op_d3(),
            0xd4 => self.op_d4(),
            0xd5 => self.op_d5(),
            0xd6 => self.op_d6(),
            0xd7 => self.op_d7(),
            0xd8 => self.op_d8(),
            0xd9 => self.op_d9(),
            0xda => self.op_da(),
            0xdb => self.op_db(),
            0xdc => self.op_dc(),
            0xdd => self.op_dd(),
            0xde => self.op_de(),
            0xdf => self.op_df(),
            0xe0 => self.op_e0(),
            0xe1 => self.op_e1(),
            0xe2 => self.op_e2(),
            0xe3 => self.op_e3(),
            0xe4 => self.op_e4(),
            0xe5 => self.op_e5(),
            0xe6 => self.op_e6(),
            0xe7 => self.op_e7(),
            0xe8 => self.op_e8(),
            0xe9 => self.op_e9(),
            0xea => self.op_ea(),
            0xeb => self.op_eb(),
            0xec => self.op_ec(),
            0xed => self.op_ed(),
            0xee => self.op_ee(),
            0xef => self.op_ef(),
            0xf0 => self.op_f0(),
            0xf1 => self.op_f1(),
            0xf2 => self.op_f2(),
            0xf3 => self.op_f3(),
            0xf4 => self.op_f4(),
            0xf5 => self.op_f5(),
            0xf6 => self.op_f6(),
            0xf7 => self.op_f7(),
            0xf8 => self.op_f8(),
            0xf9 => self.op_f9(),
            0xfa => self.op_fa(),
            0xfb => self.op_fb(),
            0xfc => self.op_fc(),
            0xfd => self.op_fd(),
            0xfe => self.op_fe(),
            0xff => self.op_ff(),
        }
    }
}

// Address modes.
impl CPU {
    fn zp(&mut self, bus: &mut Bus) {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // zero-page address
            }
            1 => {
                bus.read(bus.data as u16); // operand
            }
            _ => unreachable!(),
        }
    }

    fn abs(&mut self, bus: &mut Bus) {
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
            _ => unreachable!(),
        }
    }

    fn zpi(&mut self, bus: &mut Bus) {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // base zero-page address
            }
            1 => {
                self.vector = add_without_carry(bus.data as u16, self.x);
                bus.read(bus.data as u16); // dummy read
            }
            2 => {
                bus.read(self.vector); // operand
            }
            _ => unreachable!(),
        }
    }

    fn abi(&mut self, bus: &mut Bus, reg: u8, store: bool) {
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
                if addr & 0xff00 == self.vector & 0xff00 && !store {
                    bus.read(add_without_carry(self.vector, reg)); // operand
                    self.op_cycle += 1;
                } else {
                    bus.read(add_without_carry(self.vector, reg)); // dummy read
                }
            }
            3 => {
                bus.read(self.vector.wrapping_add(reg as u16)); // operand
            }
            _ => unreachable!(),
        }
    }

    fn izx(&mut self, bus: &mut Bus) {
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
            _ => unreachable!(),
        }
    }

    fn izy(&mut self, bus: &mut Bus, store: bool) {
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
                if addr & 0xff00 == self.vector & 0xff00 && !store {
                    bus.read(add_without_carry(self.vector, self.y)); // operand
                    self.op_cycle += 1;
                } else {
                    bus.read(add_without_carry(self.vector, self.y)); // dummy read
                }
            }
            4 => {
                bus.read(self.vector.wrapping_add(self.y as u16)); // operand
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
    fn asl(&mut self, data: u8) -> u8 {
        let val = data.wrapping_shl(1);
        self.flags.set_nz(val);
        self.flags.update_c(data & 0x80 != 0);
        val
    }

    fn and(&mut self, data: u8) {
        self.a &= data;
        self.flags.set_nz(self.a);
    }

    fn ora(&mut self, data: u8) {
        self.a |= data;
        self.flags.set_nz(self.a);
    }
}
