use crate::system::State;

/// Increment an 8-bit value by 1 and update flags accordingly
fn inc_8bit(value: u8, state: &mut State) -> u8 {
    let result = value.wrapping_add(1);
    state.set_flag_z(result == 0);
    state.set_flag_n(false);
    state.set_flag_h((value & 0xF) == 0xF);
    result
}

/// Increment the register A by 1 and update flags accordingly
pub fn inc_a(state: &mut State) {
    state.a = inc_8bit(state.a, state);
}

// Increment the register B by 1 and update flags accordingly
pub fn inc_b(state: &mut State) {
    state.b = inc_8bit(state.b, state);
}

// Increment the register C by 1 and update flags accordingly
pub fn inc_c(state: &mut State) {
    state.c = inc_8bit(state.c, state);
}

/// Increment the register D by 1 and update flags accordingly
pub fn inc_d(state: &mut State) {
    state.d = inc_8bit(state.d, state);
}

/// Increment the register E by 1 and update flags accordingly
pub fn inc_e(state: &mut State) {
    state.e = inc_8bit(state.e, state);
}

/// Increment the register H by 1 and update flags accordingly
pub fn inc_h(state: &mut State) {
    state.h = inc_8bit(state.h, state);
}

/// Increment the register L by 1 and update flags accordingly
pub fn inc_l(state: &mut State) {
    state.l = inc_8bit(state.l, state);
}

/// Decrement an 8-bit value by 1 and update flags accordingly
fn dec_8bit(value: u8, state: &mut State) -> u8 {
    let result = value.wrapping_sub(1);
    state.set_flag_z(result == 0);
    state.set_flag_n(true);
    state.set_flag_h((value & 0xF) == 0);
    result
}

/// Decrement the register A by 1 and update flags accordingly
pub fn dec_a(state: &mut State) {
    state.a = dec_8bit(state.a, state);
}

/// Decrement the register B by 1 and update flags accordingly
pub fn dec_b(state: &mut State) {
    state.b = dec_8bit(state.b, state);
}

/// Decrement the register C by 1 and update flags accordingly
pub fn dec_c(state: &mut State) {
    state.c = dec_8bit(state.c, state);
}

/// Decrement the register D by 1 and update flags accordingly
pub fn dec_d(state: &mut State) {
    state.d = dec_8bit(state.d, state);
}

/// Decrement the register E by 1 and update flags accordingly
pub fn dec_e(state: &mut State) {
    state.e = dec_8bit(state.e, state);
}

/// Decrement the register H by 1 and update flags accordingly
pub fn dec_h(state: &mut State) {
    state.h = dec_8bit(state.h, state);
}

/// Decrement the register L by 1 and update flags accordingly
pub fn dec_l(state: &mut State) {
    state.l = dec_8bit(state.l, state);
}

/// Increment the BC register pair by 1
pub fn inc_bc(state: &mut State) {
    let value = state.bc().wrapping_add(1);
    state.set_bc(value);
}

/// Increment the DE register pair by 1
pub fn inc_de(state: &mut State) {
    let value = state.de().wrapping_add(1);
    state.set_de(value);
}

/// Increment the HL register pair by 1
pub fn inc_hl(state: &mut State) {
    let value = state.hl().wrapping_add(1);
    state.set_hl(value);
}

/// Increment the SP register by 1
pub fn inc_sp(state: &mut State) {
    let value = state.sp().wrapping_add(1);
    state.set_sp(value);
}

/// Decrement the BC register pair by 1
pub fn dec_bc(state: &mut State) {
    let value = state.bc().wrapping_sub(1);
    state.set_bc(value);
}

/// Decrement the DE register pair by 1
pub fn dec_de(state: &mut State) {
    let value = state.de().wrapping_sub(1);
    state.set_de(value);
}

/// Decrement the HL register pair by 1
pub fn dec_hl(state: &mut State) {
    let value = state.hl().wrapping_sub(1);
    state.set_hl(value);
}

/// Decrement the SP register by 1
pub fn dec_sp(state: &mut State) {
    let value = state.sp().wrapping_sub(1);
    state.set_sp(value);
}

/// Execute a single CPU instruction.
pub fn execute(state: &mut State) {
    // TODO: handle interrupts

    // TODO: This is not fully correct, in fact the read function must take into consideration the
    // current emomory bank and other detalis.
    let op = state.read(state.pc);
    state.pc += 1;

    match op {
        0x00 => { /* NOP */ }
        0x01 => {
            /* LD BC,n */
            state.set_bc(state.read_word(state.pc));
            state.pc += 2;
        }
        0x02 => {
            /* LD (BC),A */
            state.write(state.bc(), state.a);
        }
        0x03 => {
            /* INC BC */
            inc_bc(state);
        }
        0x04 => {
            /* INC B */
            inc_b(state);
        }
        0x05 => {
            /* DEC B */
            dec_b(state);
        }
        0x06 => {
            /* LD B,n */
            state.b = state.read(state.pc);
            state.pc += 1;
        }
        0x07 => {
            /* RLCA */
            // TODO: Implement RLC (Rotate Left Circular) function
            unimplemented!("RLCA - needs RLC function");
        }
        0x08 => {
            /* LD (n),SP */
            // TODO: Verify write_word exists, then implement
            unimplemented!("LD (n),SP - needs write_word function");
        }
        0x09 => {
            /* ADD HL,BC */
            // TODO: Implement ADDW (16-bit add with flags) function
            unimplemented!("ADD HL,BC - needs ADDW function");
        }
        0x0A => {
            /* LD A,(BC) */
            state.a = state.read(state.bc());
        }
        0x0B => {
            /* DEC BC */
            state.set_bc(state.bc().wrapping_sub(1));
        }
        0x0C => {
            /* INC C */
            inc_c(state);
        }
        0x0D => {
            /* DEC C */
            dec_c(state);
        }
        0x0E => {
            /* LD C,n */
            state.c = state.read(state.pc);
            state.pc += 1;
        }
        0x0F => {
            /* RRCA */
            // TODO: Implement RRC (Rotate Right Circular) function
            unimplemented!("RRCA - needs RRC function");
        }
        0x10 => {
            /* STOP */
            state.pc += 1;
        }
        0x11 => {
            /* LD DE,n */
            state.set_de(state.read_word(state.pc));
            state.pc += 2;
        }
        0x12 => {
            /* LD (DE),A */
            state.write(state.de(), state.a);
        }
        0x13 => {
            /* INC DE */
            inc_de(state);
        }
        0x14 => {
            /* INC D */
            inc_d(state);
        }
        0x15 => {
            /* DEC D */
            dec_d(state);
        }
        0x16 => {
            /* LD D,n */
            state.d = state.read(state.pc);
            state.pc += 1;
        }
        _ => {
            panic!("Unimplemented opcode: 0x{:02X}", op);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inc_8bit_normal() {
        let mut state = State::new();
        state.a = 0x42;

        inc_a(&mut state);

        assert_eq!(state.a, 0x43);
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
    }

    #[test]
    fn test_inc_8bit_zero() {
        let mut state = State::new();
        state.b = 0xFF;

        inc_b(&mut state);

        assert_eq!(state.b, 0x00);
        assert!(state.flag_z()); // Result is zero
        assert!(!state.flag_n());
        assert!(state.flag_h()); // Half carry from 0xF to 0x0
    }

    #[test]
    fn test_inc_8bit_half_carry() {
        let mut state = State::new();
        state.c = 0x0F;

        inc_c(&mut state);

        assert_eq!(state.c, 0x10);
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(state.flag_h()); // Half carry from bit 3 to bit 4
    }

    #[test]
    fn test_inc_8bit_all_registers() {
        let mut state = State::new();

        state.a = 0x00;
        inc_a(&mut state);
        assert_eq!(state.a, 0x01);
        state.b = 0x00;
        inc_b(&mut state);
        assert_eq!(state.b, 0x01);
        state.c = 0x00;
        inc_c(&mut state);
        assert_eq!(state.c, 0x01);
        state.d = 0x00;
        inc_d(&mut state);
        assert_eq!(state.d, 0x01);
        state.e = 0x00;
        inc_e(&mut state);
        assert_eq!(state.e, 0x01);
        state.h = 0x00;
        inc_h(&mut state);
        assert_eq!(state.h, 0x01);
        state.l = 0x00;
        inc_l(&mut state);
        assert_eq!(state.l, 0x01);
    }

    #[test]
    fn test_inc_16bit_normal() {
        let mut state = State::new();
        state.set_bc(0x1234);

        inc_bc(&mut state);

        assert_eq!(state.bc(), 0x1235);
    }

    #[test]
    fn test_inc_16bit_overflow() {
        let mut state = State::new();
        state.set_de(0xFFFF);

        inc_de(&mut state);

        assert_eq!(state.de(), 0x0000);
    }

    #[test]
    fn test_inc_16bit_all_registers() {
        let mut state = State::new();

        state.set_bc(0x0000);
        inc_bc(&mut state);
        assert_eq!(state.bc(), 0x0001);
        state.set_de(0x0000);
        inc_de(&mut state);
        assert_eq!(state.de(), 0x0001);
        state.set_hl(0x0000);
        inc_hl(&mut state);
        assert_eq!(state.hl(), 0x0001);
        state.set_sp(0x0000);
        inc_sp(&mut state);
        assert_eq!(state.sp(), 0x0001);
    }

    #[test]
    fn test_dec_8bit_normal() {
        let mut state = State::new();
        state.a = 0x42;

        dec_a(&mut state);

        assert_eq!(state.a, 0x41);
        assert!(!state.flag_z());
        assert!(state.flag_n()); // N flag is set for subtraction
        assert!(!state.flag_h());
    }

    #[test]
    fn test_dec_8bit_zero() {
        let mut state = State::new();
        state.b = 0x01;

        dec_b(&mut state);

        assert_eq!(state.b, 0x00);
        assert!(state.flag_z()); // Result is zero
        assert!(state.flag_n());
        assert!(!state.flag_h()); // No half-borrow
    }

    #[test]
    fn test_dec_8bit_half_borrow() {
        let mut state = State::new();
        state.c = 0x10;

        dec_c(&mut state);

        assert_eq!(state.c, 0x0F);
        assert!(!state.flag_z());
        assert!(state.flag_n());
        assert!(state.flag_h()); // Half borrow from bit 4 to bit 3
    }

    #[test]
    fn test_dec_8bit_underflow() {
        let mut state = State::new();
        state.d = 0x00;

        dec_d(&mut state);

        assert_eq!(state.d, 0xFF);
        assert!(!state.flag_z());
        assert!(state.flag_n());
        assert!(state.flag_h()); // Half borrow when decrementing from 0
    }

    #[test]
    fn test_dec_8bit_all_registers() {
        let mut state = State::new();

        state.a = 0x02;
        dec_a(&mut state);
        assert_eq!(state.a, 0x01);
        state.b = 0x02;
        dec_b(&mut state);
        assert_eq!(state.b, 0x01);
        state.c = 0x02;
        dec_c(&mut state);
        assert_eq!(state.c, 0x01);
        state.d = 0x02;
        dec_d(&mut state);
        assert_eq!(state.d, 0x01);
        state.e = 0x02;
        dec_e(&mut state);
        assert_eq!(state.e, 0x01);
        state.h = 0x02;
        dec_h(&mut state);
        assert_eq!(state.h, 0x01);
        state.l = 0x02;
        dec_l(&mut state);
        assert_eq!(state.l, 0x01);
    }

    #[test]
    fn test_dec_16bit_normal() {
        let mut state = State::new();
        state.set_bc(0x1234);

        dec_bc(&mut state);

        assert_eq!(state.bc(), 0x1233);
    }

    #[test]
    fn test_dec_16bit_underflow() {
        let mut state = State::new();
        state.set_de(0x0000);

        dec_de(&mut state);

        assert_eq!(state.de(), 0xFFFF);
    }

    #[test]
    fn test_dec_16bit_all_registers() {
        let mut state = State::new();

        state.set_bc(0x0002);
        dec_bc(&mut state);
        assert_eq!(state.bc(), 0x0001);
        state.set_de(0x0002);
        dec_de(&mut state);
        assert_eq!(state.de(), 0x0001);
        state.set_hl(0x0002);
        dec_hl(&mut state);
        assert_eq!(state.hl(), 0x0001);
        state.set_sp(0x0002);
        dec_sp(&mut state);
        assert_eq!(state.sp(), 0x0001);
    }
}
