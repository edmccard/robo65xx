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
                    self.read_sp(bus); // dummy read
                    self.sp = self.sp.wrapping_sub(1);
                } else {
                    self.write_sp(bus, (self.pc >> 8) as u8);
                }
            }
            2 => {
                if ctrl.res {
                    self.read_sp(bus); // dummy read
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
                    self.read_sp(bus); // dummy read
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

    fn op_01(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.izx_alu(Self::ora, bus, ctrl);
    }

    fn op_02(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.kil(bus, ctrl);
    }

    fn op_03(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.izx_rmw(Self::slo, bus, ctrl);
    }

    fn op_04(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zp_alu(Self::nop, bus, ctrl);
    }

    fn op_05(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zp_alu(Self::ora, bus, ctrl);
    }

    fn op_06(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zp_rmw(Self::asl, bus, ctrl);
    }

    fn op_07(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zp_rmw(Self::slo, bus, ctrl);
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

    fn op_09(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.imm_alu(Self::ora, bus, ctrl);
    }

    fn op_0a(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.implicit(|cpu| cpu.a = cpu.asl(cpu.a), bus, ctrl);
    }

    fn op_0b(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.imm_alu(Self::anc, bus, ctrl);
    }

    fn op_0c(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abs_alu(Self::nop, bus, ctrl);
    }

    fn op_0d(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abs_alu(Self::ora, bus, ctrl);
    }

    fn op_0e(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abs_rmw(Self::asl, bus, ctrl);
    }

    fn op_0f(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abs_rmw(Self::slo, bus, ctrl);
    }

    fn op_10(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.branch(bus, ctrl, !self.flags.n());
    }

    fn op_11(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.izy_alu(Self::ora, bus, ctrl);
    }

    fn op_12(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.kil(bus, ctrl);
    }

    fn op_13(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.izy_rmw(Self::slo, bus, ctrl);
    }

    fn op_14(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zpi_alu(self.x, Self::nop, bus, ctrl);
    }

    fn op_15(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zpi_alu(self.x, Self::ora, bus, ctrl);
    }

    fn op_16(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zpi_rmw(self.x, Self::asl, bus, ctrl);
    }

    fn op_17(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zpi_rmw(self.x, Self::slo, bus, ctrl);
    }

    fn op_18(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.implicit(|cpu| cpu.flags.update_c(false), bus, ctrl);
    }

    fn op_19(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_alu(self.y, Self::ora, bus, ctrl);
    }

    fn op_1a(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.implicit(|_| {}, bus, ctrl);
    }

    fn op_1b(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_rmw(self.y, Self::slo, bus, ctrl);
    }

    fn op_1c(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_alu(self.x, Self::nop, bus, ctrl);
    }

    fn op_1d(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_alu(self.x, Self::ora, bus, ctrl);
    }

    fn op_1e(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_rmw(self.x, Self::asl, bus, ctrl);
    }

    fn op_1f(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_rmw(self.x, Self::slo, bus, ctrl);
    }

    // JSR
    fn op_20(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // low byte of target
            }
            1 => {
                self.vector = bus.data as u16;
                self.read_sp(bus); // dummy read
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

    fn op_21(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.izx_alu(Self::and, bus, ctrl);
    }

    fn op_22(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.kil(bus, ctrl);
    }

    fn op_23(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.izx_rmw(Self::rla, bus, ctrl);
    }

    fn op_24(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zp_alu(Self::bit, bus, ctrl);
    }

    fn op_25(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zp_alu(Self::and, bus, ctrl);
    }

    fn op_26(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zp_rmw(Self::rol, bus, ctrl);
    }

    fn op_27(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zp_rmw(Self::rla, bus, ctrl);
    }

    // PLP
    fn op_28(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                bus.read(self.pc); // dummy read
            }
            1 => {
                self.read_sp(bus); // dummy read
                self.sp = self.sp.wrapping_add(1);
            }
            2 => {
                self.read_sp(bus); // data
                self.poll_intg(ctrl);
            }
            3 => {
                self.flags.set_from(bus.data, 0b11111111);
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn op_29(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.imm_alu(Self::and, bus, ctrl);
    }

    fn op_2a(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.implicit(|cpu| cpu.a = cpu.rol(cpu.a), bus, ctrl);
    }

    fn op_2b(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.imm_alu(Self::anc, bus, ctrl);
    }

    fn op_2c(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abs_alu(Self::bit, bus, ctrl);
    }

    fn op_2d(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abs_alu(Self::and, bus, ctrl);
    }

    fn op_2e(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abs_rmw(Self::rol, bus, ctrl);
    }

    fn op_2f(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abs_rmw(Self::rla, bus, ctrl);
    }

    fn op_30(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.branch(bus, ctrl, self.flags.n());
    }

    fn op_31(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.izy_alu(Self::and, bus, ctrl);
    }

    fn op_32(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.kil(bus, ctrl);
    }

    fn op_33(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.izy_rmw(Self::rla, bus, ctrl);
    }

    fn op_34(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zpi_alu(self.x, Self::nop, bus, ctrl);
    }

    fn op_35(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zpi_alu(self.x, Self::and, bus, ctrl);
    }

    fn op_36(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zpi_rmw(self.x, Self::rol, bus, ctrl);
    }

    fn op_37(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zpi_rmw(self.x, Self::rla, bus, ctrl);
    }

    fn op_38(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.implicit(|cpu| cpu.flags.update_c(true), bus, ctrl);
    }

    fn op_39(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_alu(self.y, Self::and, bus, ctrl);
    }

    fn op_3a(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.implicit(|_| {}, bus, ctrl);
    }

    fn op_3b(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_rmw(self.y, Self::rla, bus, ctrl);
    }

    fn op_3c(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_alu(self.x, Self::nop, bus, ctrl);
    }

    fn op_3d(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_alu(self.x, Self::and, bus, ctrl);
    }

    fn op_3e(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_rmw(self.x, Self::rol, bus, ctrl);
    }

    fn op_3f(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_rmw(self.x, Self::rla, bus, ctrl);
    }

    // RTI
    fn op_40(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                bus.read(self.pc); // dummy read
            }
            1 => {
                self.read_sp(bus); // dummy read
                self.sp = self.sp.wrapping_add(1);
            }
            2 => {
                self.read_sp(bus);
                self.sp = self.sp.wrapping_add(1);
            }
            3 => {
                self.flags.set_from(bus.data, 0b11111111);
                self.read_sp(bus); // low byte of return address
                self.sp = self.sp.wrapping_add(1);
            }
            4 => {
                self.vector = bus.data as u16;
                self.read_sp(bus); // high byte of return address
                self.poll_intg(ctrl);
            }
            5 => {
                self.pc = (bus.data as u16).wrapping_shl(8) | self.vector;
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn op_41(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.izx_alu(Self::eor, bus, ctrl);
    }

    fn op_42(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.kil(bus, ctrl);
    }

    fn op_43(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.izx_rmw(Self::sre, bus, ctrl);
    }

    fn op_44(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zp_alu(Self::nop, bus, ctrl);
    }

    fn op_45(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zp_alu(Self::eor, bus, ctrl);
    }

    fn op_46(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zp_rmw(Self::lsr, bus, ctrl);
    }

    fn op_47(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zp_rmw(Self::sre, bus, ctrl);
    }

    // PHA
    fn op_48(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                bus.read(self.pc); // dummy read
            }
            1 => {
                self.write_sp(bus, self.a);
                self.poll_intg(ctrl);
            }
            2 => {
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn op_49(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.imm_alu(Self::eor, bus, ctrl);
    }

    fn op_4a(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.implicit(|cpu| cpu.a = cpu.lsr(cpu.a), bus, ctrl);
    }

    fn op_4b(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.imm_alu(
            |cpu, val| {
                cpu.a &= val;
                cpu.a = cpu.lsr(cpu.a)
            },
            bus,
            ctrl,
        );
    }

    // JMP
    fn op_4c(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // low byte of target address
            }
            1 => {
                self.vector = bus.data as u16;
                self.read_pc(bus); // high byte of target address
                self.poll_intg(ctrl);
            }
            2 => {
                self.pc = (bus.data as u16).wrapping_shl(8) | self.vector;
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn op_4d(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abs_alu(Self::eor, bus, ctrl);
    }

    fn op_4e(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abs_rmw(Self::lsr, bus, ctrl);
    }

    fn op_4f(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abs_rmw(Self::sre, bus, ctrl);
    }

    fn op_50(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.branch(bus, ctrl, !self.flags.v());
    }

    fn op_51(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.izy_alu(Self::eor, bus, ctrl);
    }

    fn op_52(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.kil(bus, ctrl);
    }

    fn op_53(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.izy_rmw(Self::sre, bus, ctrl);
    }

    fn op_54(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zpi_alu(self.x, Self::nop, bus, ctrl);
    }

    fn op_55(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zpi_alu(self.x, Self::eor, bus, ctrl);
    }

    fn op_56(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zpi_rmw(self.x, Self::lsr, bus, ctrl);
    }

    fn op_57(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zpi_rmw(self.x, Self::sre, bus, ctrl);
    }

    fn op_58(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.implicit(|cpu| cpu.flags.update_i(false), bus, ctrl);
    }

    fn op_59(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_alu(self.y, Self::eor, bus, ctrl);
    }

    fn op_5a(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.implicit(|_| {}, bus, ctrl);
    }

    fn op_5b(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_rmw(self.y, Self::sre, bus, ctrl);
    }

    fn op_5c(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_alu(self.x, Self::nop, bus, ctrl);
    }

    fn op_5d(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_alu(self.x, Self::eor, bus, ctrl);
    }

    fn op_5e(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_rmw(self.x, Self::lsr, bus, ctrl);
    }

    fn op_5f(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_rmw(self.x, Self::sre, bus, ctrl);
    }

    // RTS
    fn op_60(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                bus.read(self.pc); // dummy read
            }
            1 => {
                self.read_sp(bus); // dummy read
                self.sp = self.sp.wrapping_add(1);
            }
            2 => {
                self.read_sp(bus); // low byte of return address
                self.sp = self.sp.wrapping_add(1);
            }
            3 => {
                self.vector = bus.data as u16;
                self.read_sp(bus); // high byte of return address
            }
            4 => {
                self.pc = (bus.data as u16).wrapping_shl(8) | self.vector;
                self.read_pc(bus); // dummy read
                self.poll_intg(ctrl);
            }
            5 => {
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn op_61(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.izx_alu(Self::adc, bus, ctrl);
    }

    fn op_62(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.kil(bus, ctrl);
    }

    fn op_63(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.izx_rmw(Self::rra, bus, ctrl);
    }

    fn op_64(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zp_alu(Self::nop, bus, ctrl);
    }

    fn op_65(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zp_alu(Self::adc, bus, ctrl);
    }

    fn op_66(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zp_rmw(Self::ror, bus, ctrl);
    }

    fn op_67(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zp_rmw(Self::rra, bus, ctrl);
    }

    // PLA
    fn op_68(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                bus.read(self.pc); // dummy read
            }
            1 => {
                self.read_sp(bus); // dummy read
                self.sp = self.sp.wrapping_add(1);
            }
            2 => {
                self.read_sp(bus); // data
                self.poll_intg(ctrl);
            }
            3 => {
                self.a = bus.data;
                self.flags.set_nz(self.a);
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn op_69(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.imm_alu(Self::adc, bus, ctrl);
    }

    fn op_6a(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.implicit(|cpu| cpu.a = cpu.ror(cpu.a), bus, ctrl);
    }

    fn op_6b(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.imm_alu(Self::arr, bus, ctrl);
    }

    // JMP ($nnnn)
    fn op_6c(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        match self.op_cycle {
            0 => {
                self.read_pc(bus); // low byte of pointer address
            }
            1 => {
                self.vector = bus.data as u16;
                self.read_pc(bus); // high byte of pointer address
            }
            2 => {
                self.vector = (bus.data as u16).wrapping_shl(8) | self.vector;
                bus.read(self.vector); // low byte of target address
            }
            3 => {
                bus.read(add_without_carry(self.vector, 1)); // high byte of target address
                self.vector = bus.data as u16;
                self.poll_intg(ctrl);
            }
            4 => {
                self.pc = (bus.data as u16).wrapping_shl(8) | self.vector;
                self.fetch(bus, ctrl);
            }
            _ => unreachable!(),
        }
    }

    fn op_6d(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abs_alu(Self::adc, bus, ctrl);
    }

    fn op_6e(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abs_rmw(Self::ror, bus, ctrl);
    }

    fn op_6f(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abs_rmw(Self::rra, bus, ctrl);
    }

    fn op_70(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.branch(bus, ctrl, self.flags.v());
    }

    fn op_71(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.izy_alu(Self::adc, bus, ctrl);
    }

    fn op_72(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.kil(bus, ctrl);
    }

    fn op_73(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.izy_rmw(Self::rra, bus, ctrl);
    }

    fn op_74(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zpi_alu(self.x, Self::nop, bus, ctrl);
    }

    fn op_75(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zpi_alu(self.x, Self::adc, bus, ctrl);
    }

    fn op_76(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zpi_rmw(self.x, Self::ror, bus, ctrl);
    }

    fn op_77(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.zpi_rmw(self.x, Self::rra, bus, ctrl);
    }

    fn op_78(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.implicit(|cpu| cpu.flags.update_i(true), bus, ctrl);
    }

    fn op_79(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_alu(self.y, Self::adc, bus, ctrl);
    }

    fn op_7a(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.implicit(|_| {}, bus, ctrl);
    }

    fn op_7b(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_rmw(self.y, Self::rra, bus, ctrl);
    }

    fn op_7c(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_alu(self.x, Self::nop, bus, ctrl);
    }

    fn op_7d(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_alu(self.x, Self::adc, bus, ctrl);
    }

    fn op_7e(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_rmw(self.x, Self::ror, bus, ctrl);
    }

    fn op_7f(&mut self, bus: &mut Bus, ctrl: &mut Control) {
        self.abi_rmw(self.x, Self::rra, bus, ctrl);
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
            0x20 => self.op_20(bus, ctrl),
            0x21 => self.op_21(bus, ctrl),
            0x22 => self.op_22(bus, ctrl),
            0x23 => self.op_23(bus, ctrl),
            0x24 => self.op_24(bus, ctrl),
            0x25 => self.op_25(bus, ctrl),
            0x26 => self.op_26(bus, ctrl),
            0x27 => self.op_27(bus, ctrl),
            0x28 => self.op_28(bus, ctrl),
            0x29 => self.op_29(bus, ctrl),
            0x2a => self.op_2a(bus, ctrl),
            0x2b => self.op_2b(bus, ctrl),
            0x2c => self.op_2c(bus, ctrl),
            0x2d => self.op_2d(bus, ctrl),
            0x2e => self.op_2e(bus, ctrl),
            0x2f => self.op_2f(bus, ctrl),
            0x30 => self.op_30(bus, ctrl),
            0x31 => self.op_31(bus, ctrl),
            0x32 => self.op_32(bus, ctrl),
            0x33 => self.op_33(bus, ctrl),
            0x34 => self.op_34(bus, ctrl),
            0x35 => self.op_35(bus, ctrl),
            0x36 => self.op_36(bus, ctrl),
            0x37 => self.op_37(bus, ctrl),
            0x38 => self.op_38(bus, ctrl),
            0x39 => self.op_39(bus, ctrl),
            0x3a => self.op_3a(bus, ctrl),
            0x3b => self.op_3b(bus, ctrl),
            0x3c => self.op_3c(bus, ctrl),
            0x3d => self.op_3d(bus, ctrl),
            0x3e => self.op_3e(bus, ctrl),
            0x3f => self.op_3f(bus, ctrl),
            0x40 => self.op_40(bus, ctrl),
            0x41 => self.op_41(bus, ctrl),
            0x42 => self.op_42(bus, ctrl),
            0x43 => self.op_43(bus, ctrl),
            0x44 => self.op_44(bus, ctrl),
            0x45 => self.op_45(bus, ctrl),
            0x46 => self.op_46(bus, ctrl),
            0x47 => self.op_47(bus, ctrl),
            0x48 => self.op_48(bus, ctrl),
            0x49 => self.op_49(bus, ctrl),
            0x4a => self.op_4a(bus, ctrl),
            0x4b => self.op_4b(bus, ctrl),
            0x4c => self.op_4c(bus, ctrl),
            0x4d => self.op_4d(bus, ctrl),
            0x4e => self.op_4e(bus, ctrl),
            0x4f => self.op_4f(bus, ctrl),
            0x50 => self.op_50(bus, ctrl),
            0x51 => self.op_51(bus, ctrl),
            0x52 => self.op_52(bus, ctrl),
            0x53 => self.op_53(bus, ctrl),
            0x54 => self.op_54(bus, ctrl),
            0x55 => self.op_55(bus, ctrl),
            0x56 => self.op_56(bus, ctrl),
            0x57 => self.op_57(bus, ctrl),
            0x58 => self.op_58(bus, ctrl),
            0x59 => self.op_59(bus, ctrl),
            0x5a => self.op_5a(bus, ctrl),
            0x5b => self.op_5b(bus, ctrl),
            0x5c => self.op_5c(bus, ctrl),
            0x5d => self.op_5d(bus, ctrl),
            0x5e => self.op_5e(bus, ctrl),
            0x5f => self.op_5f(bus, ctrl),
            0x60 => self.op_60(bus, ctrl),
            0x61 => self.op_61(bus, ctrl),
            0x62 => self.op_62(bus, ctrl),
            0x63 => self.op_63(bus, ctrl),
            0x64 => self.op_64(bus, ctrl),
            0x65 => self.op_65(bus, ctrl),
            0x66 => self.op_66(bus, ctrl),
            0x67 => self.op_67(bus, ctrl),
            0x68 => self.op_68(bus, ctrl),
            0x69 => self.op_69(bus, ctrl),
            0x6a => self.op_6a(bus, ctrl),
            0x6b => self.op_6b(bus, ctrl),
            0x6c => self.op_6c(bus, ctrl),
            0x6d => self.op_6d(bus, ctrl),
            0x6e => self.op_6e(bus, ctrl),
            0x6f => self.op_6f(bus, ctrl),
            0x70 => self.op_70(bus, ctrl),
            0x71 => self.op_71(bus, ctrl),
            0x72 => self.op_72(bus, ctrl),
            0x73 => self.op_73(bus, ctrl),
            0x74 => self.op_74(bus, ctrl),
            0x75 => self.op_75(bus, ctrl),
            0x76 => self.op_76(bus, ctrl),
            0x77 => self.op_77(bus, ctrl),
            0x78 => self.op_78(bus, ctrl),
            0x79 => self.op_79(bus, ctrl),
            0x7a => self.op_7a(bus, ctrl),
            0x7b => self.op_7b(bus, ctrl),
            0x7c => self.op_7c(bus, ctrl),
            0x7d => self.op_7d(bus, ctrl),
            0x7e => self.op_7e(bus, ctrl),
            0x7f => self.op_7f(bus, ctrl),
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

// Common operations
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
        if self.op_cycle < 2 {
            self.zp(bus);
            if self.op_cycle == 1 {
                self.poll_intg(ctrl);
            }
        } else {
            op(self, bus.data);
            self.fetch(bus, ctrl);
        }
    }

    fn zp_rmw<F>(&mut self, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8) -> u8,
    {
        if self.op_cycle < 2 {
            self.zp(bus);
        } else {
            match self.op_cycle {
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
    }

    fn abs_alu<F>(&mut self, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8),
    {
        if self.op_cycle < 3 {
            self.abs(bus);
            if self.op_cycle == 2 {
                self.poll_intg(ctrl);
            }
        } else {
            op(self, bus.data);
            self.fetch(bus, ctrl);
        }
    }

    fn abs_rmw<F>(&mut self, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8) -> u8,
    {
        if self.op_cycle < 3 {
            self.abs(bus);
        } else {
            match self.op_cycle {
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
    }

    fn zpi_alu<F>(&mut self, reg: u8, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8),
    {
        if self.op_cycle < 3 {
            self.zpi(bus, reg);
            if self.op_cycle == 2 {
                self.poll_intg(ctrl);
            }
        } else {
            op(self, bus.data);
            self.fetch(bus, ctrl);
        }
    }

    fn zpi_rmw<F>(&mut self, reg: u8, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8) -> u8,
    {
        if self.op_cycle < 3 {
            self.zpi(bus, reg);
        } else {
            match self.op_cycle {
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
    }

    fn abi_alu<F>(&mut self, reg: u8, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8),
    {
        if self.op_cycle < 4 {
            self.abi(bus, reg, false);
            if self.op_cycle == 3 {
                self.poll_intg(ctrl);
            }
        } else {
            op(self, bus.data);
            self.fetch(bus, ctrl);
        }
    }

    fn abi_rmw<F>(&mut self, reg: u8, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8) -> u8,
    {
        if self.op_cycle < 4 {
            self.abi(bus, reg, true);
        } else {
            match self.op_cycle {
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
    }

    fn izx_alu<F>(&mut self, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8),
    {
        if self.op_cycle < 5 {
            self.izx(bus);
            if self.op_cycle == 4 {
                self.poll_intg(ctrl);
            }
        } else {
            op(self, bus.data);
            self.fetch(bus, ctrl);
        }
    }

    fn izx_rmw<F>(&mut self, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8) -> u8,
    {
        if self.op_cycle < 5 {
            self.izx(bus);
        } else {
            match self.op_cycle {
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
    }

    fn izy_alu<F>(&mut self, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8),
    {
        if self.op_cycle < 5 {
            self.izy(bus, false);
            if self.op_cycle == 4 {
                self.poll_intg(ctrl);
            }
        } else {
            op(self, bus.data);
            self.fetch(bus, ctrl);
        }
    }

    fn izy_rmw<F>(&mut self, op: F, bus: &mut Bus, ctrl: &mut Control)
    where
        F: Fn(&mut Self, u8) -> u8,
    {
        if self.op_cycle < 5 {
            self.izy(bus, true);
        } else {
            match self.op_cycle {
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

    fn zpi(&mut self, bus: &mut Bus, reg: u8) {
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
    fn nop(&mut self, _: u8) {}

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
}
