use crate::io::{IE, IF};
use crate::system::State;

/// Check if there are any pending interrupts that should wake the CPU
fn has_pending_interrupt(state: &State) -> bool {
    let ie = state.read(IE); // Interrupt Enable
    let if_flags = state.read(IF); // Interrupt Flags

    // Check if any enabled interrupt has its flag set
    (ie & if_flags & 0x1F) != 0
}

/// Service pending interrupts if IME is enabled
/// Returns true if an interrupt was serviced
fn service_interrupts(state: &mut State) -> bool {
    if !state.ime {
        return false;
    }

    let ie = state.read(IE);
    let if_flags = state.read(IF);
    let pending = ie & if_flags & 0x1F;

    if pending == 0 {
        return false;
    }

    // Find the highest priority interrupt (lowest bit number)
    // Priority: V-Blank (bit 0) > LCD STAT (bit 1) > Timer (bit 2) > Serial (bit 3) > Joypad (bit 4)
    let interrupt_bit = pending.trailing_zeros();

    // Interrupt vectors
    let vector = match interrupt_bit {
        0 => 0x0040,       // V-Blank
        1 => 0x0048,       // LCD STAT
        2 => 0x0050,       // Timer
        3 => 0x0058,       // Serial
        4 => 0x0060,       // Joypad
        _ => return false, // Should never happen
    };

    // Disable IME
    state.ime = false;

    // Exit HALT mode if CPU was halted
    state.halt = false;

    // Clear the interrupt flag
    let new_if = if_flags & !(1 << interrupt_bit);
    state.write(IF, new_if);

    // Push PC onto stack
    state.sp = state.sp.wrapping_sub(2);
    state.write(state.sp, (state.pc & 0xFF) as u8);
    state.write(state.sp.wrapping_add(1), (state.pc >> 8) as u8);

    // Jump to interrupt vector
    state.pc = vector;

    // Interrupt servicing takes 20 cycles
    state.cycles += 20;

    true
}

/// Add an 8-bit value to register A and update flags accordingly
/// Z: Set if result is zero
/// N: Reset (addition operation)
/// H: Set if carry from bit 3
/// C: Set if carry from bit 7
fn add_a(value: u8, state: &mut State) {
    let a = state.a;
    let result = a.wrapping_add(value);

    state.set_flag_z(result == 0);
    state.set_flag_n(false);
    state.set_flag_h((a & 0xF) + (value & 0xF) > 0xF);
    state.set_flag_c((a as u16) + (value as u16) > 0xFF);

    state.a = result;
}

/// Add an 8-bit value plus carry flag to register A and update flags accordingly
/// Z: Set if result is zero
/// N: Reset (addition operation)
/// H: Set if carry from bit 3
/// C: Set if carry from bit 7
fn adc_a(value: u8, state: &mut State) {
    let a = state.a;
    let carry = if state.flag_c() { 1 } else { 0 };
    let result = a.wrapping_add(value).wrapping_add(carry);

    state.set_flag_z(result == 0);
    state.set_flag_n(false);
    state.set_flag_h((a & 0xF) + (value & 0xF) + carry > 0xF);
    state.set_flag_c((a as u16) + (value as u16) + (carry as u16) > 0xFF);

    state.a = result;
}

/// Subtract an 8-bit value from register A and update flags accordingly
/// Z: Set if result is zero
/// N: Set (subtraction operation)
/// H: Set if borrow from bit 4
/// C: Set if borrow (A < value)
fn sub_a(value: u8, state: &mut State) {
    let a = state.a;
    let result = a.wrapping_sub(value);

    state.set_flag_z(result == 0);
    state.set_flag_n(true);
    state.set_flag_h((a & 0xF) < (value & 0xF));
    state.set_flag_c(a < value);

    state.a = result;
}

/// Subtract an 8-bit value plus carry flag from register A and update flags accordingly
/// Z: Set if result is zero
/// N: Set (subtraction operation)
/// H: Set if borrow from bit 4
/// C: Set if borrow
fn sbc_a(value: u8, state: &mut State) {
    let a = state.a;
    let carry = if state.flag_c() { 1 } else { 0 };
    let result = a.wrapping_sub(value).wrapping_sub(carry);

    state.set_flag_z(result == 0);
    state.set_flag_n(true);
    state.set_flag_h((a & 0xF) < (value & 0xF) + carry);
    state.set_flag_c((a as u16) < (value as u16) + (carry as u16));

    state.a = result;
}

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

/// Rotate left circular (RLC) - rotates value left, bit 7 goes to carry and bit 0
fn rlc_8bit(value: u8, state: &mut State) -> u8 {
    let bit7 = (value & 0x80) != 0;
    let result = (value << 1) | (if bit7 { 1 } else { 0 });

    state.set_flag_z(result == 0);
    state.set_flag_n(false);
    state.set_flag_h(false);
    state.set_flag_c(bit7);

    result
}

/// Rotate register A left circular
pub fn rlc_a(state: &mut State) {
    state.a = rlc_8bit(state.a, state);
}

/// Rotate register B left circular
pub fn rlc_b(state: &mut State) {
    state.b = rlc_8bit(state.b, state);
}

/// Rotate register C left circular
pub fn rlc_c(state: &mut State) {
    state.c = rlc_8bit(state.c, state);
}

/// Rotate register D left circular
pub fn rlc_d(state: &mut State) {
    state.d = rlc_8bit(state.d, state);
}

/// Rotate register E left circular
pub fn rlc_e(state: &mut State) {
    state.e = rlc_8bit(state.e, state);
}

/// Rotate register H left circular
pub fn rlc_h(state: &mut State) {
    state.h = rlc_8bit(state.h, state);
}

/// Rotate register L left circular
pub fn rlc_l(state: &mut State) {
    state.l = rlc_8bit(state.l, state);
}

/// RLCA - Rotate A left circular (always resets Z flag)
pub fn rlca(state: &mut State) {
    state.a = rlc_8bit(state.a, state);
    state.set_flag_z(false); // RLCA always resets Z flag
}

/// Rotate right circular (RRC) - rotates value right, bit 0 goes to carry and bit 7
fn rrc_8bit(value: u8, state: &mut State) -> u8 {
    let bit0 = (value & 0x01) != 0;
    let result = (value >> 1) | (if bit0 { 0x80 } else { 0 });

    state.set_flag_z(result == 0);
    state.set_flag_n(false);
    state.set_flag_h(false);
    state.set_flag_c(bit0);

    result
}

/// Rotate register A right circular
pub fn rrc_a(state: &mut State) {
    state.a = rrc_8bit(state.a, state);
}

/// Rotate register B right circular
pub fn rrc_b(state: &mut State) {
    state.b = rrc_8bit(state.b, state);
}

/// Rotate register C right circular
pub fn rrc_c(state: &mut State) {
    state.c = rrc_8bit(state.c, state);
}

/// Rotate register D right circular
pub fn rrc_d(state: &mut State) {
    state.d = rrc_8bit(state.d, state);
}

/// Rotate register E right circular
pub fn rrc_e(state: &mut State) {
    state.e = rrc_8bit(state.e, state);
}

/// Rotate register H right circular
pub fn rrc_h(state: &mut State) {
    state.h = rrc_8bit(state.h, state);
}

/// Rotate register L right circular
pub fn rrc_l(state: &mut State) {
    state.l = rrc_8bit(state.l, state);
}

/// RRCA - Rotate A right circular (always resets Z flag)
pub fn rrca(state: &mut State) {
    state.a = rrc_8bit(state.a, state);
    state.set_flag_z(false); // RRCA always resets Z flag
}

/// Rotate left through carry (RL) - rotates value left through carry flag
/// Old carry goes to bit 0, bit 7 goes to carry
fn rl_8bit(value: u8, state: &mut State) -> u8 {
    let bit7 = (value & 0x80) != 0;
    let old_carry = if state.flag_c() { 1 } else { 0 };
    let result = (value << 1) | old_carry;

    state.set_flag_z(result == 0);
    state.set_flag_n(false);
    state.set_flag_h(false);
    state.set_flag_c(bit7);

    result
}

/// Rotate register A left through carry
pub fn rl_a(state: &mut State) {
    state.a = rl_8bit(state.a, state);
}

/// Rotate register B left through carry
pub fn rl_b(state: &mut State) {
    state.b = rl_8bit(state.b, state);
}

/// Rotate register C left through carry
pub fn rl_c(state: &mut State) {
    state.c = rl_8bit(state.c, state);
}

/// Rotate register D left through carry
pub fn rl_d(state: &mut State) {
    state.d = rl_8bit(state.d, state);
}

/// Rotate register E left through carry
pub fn rl_e(state: &mut State) {
    state.e = rl_8bit(state.e, state);
}

/// Rotate register H left through carry
pub fn rl_h(state: &mut State) {
    state.h = rl_8bit(state.h, state);
}

/// Rotate register L left through carry
pub fn rl_l(state: &mut State) {
    state.l = rl_8bit(state.l, state);
}

/// RLA - Rotate A left through carry (always resets Z flag)
pub fn rla(state: &mut State) {
    state.a = rl_8bit(state.a, state);
    state.set_flag_z(false); // RLA always resets Z flag
}

/// Rotate right through carry (RR) - rotates value right through carry flag
/// Old carry goes to bit 7, bit 0 goes to carry
fn rr_8bit(value: u8, state: &mut State) -> u8 {
    let bit0 = (value & 0x01) != 0;
    let old_carry = if state.flag_c() { 0x80 } else { 0 };
    let result = (value >> 1) | old_carry;

    state.set_flag_z(result == 0);
    state.set_flag_n(false);
    state.set_flag_h(false);
    state.set_flag_c(bit0);

    result
}

/// Rotate register A right through carry
pub fn rr_a(state: &mut State) {
    state.a = rr_8bit(state.a, state);
}

/// Rotate register B right through carry
pub fn rr_b(state: &mut State) {
    state.b = rr_8bit(state.b, state);
}

/// Rotate register C right through carry
pub fn rr_c(state: &mut State) {
    state.c = rr_8bit(state.c, state);
}

/// Rotate register D right through carry
pub fn rr_d(state: &mut State) {
    state.d = rr_8bit(state.d, state);
}

/// Rotate register E right through carry
pub fn rr_e(state: &mut State) {
    state.e = rr_8bit(state.e, state);
}

/// Rotate register H right through carry
pub fn rr_h(state: &mut State) {
    state.h = rr_8bit(state.h, state);
}

/// Rotate register L right through carry
pub fn rr_l(state: &mut State) {
    state.l = rr_8bit(state.l, state);
}

/// RRA - Rotate A right through carry (always resets Z flag)
pub fn rra(state: &mut State) {
    state.a = rr_8bit(state.a, state);
    state.set_flag_z(false); // RRA always resets Z flag
}

/// JR - Jump relative (unconditional)
/// Adds a signed 8-bit offset to PC
pub fn jr(state: &mut State) {
    let offset = state.read(state.pc) as i8;
    state.pc += 1;
    // Add the signed offset to PC
    state.pc = state.pc.wrapping_add(offset as u16);
}

/// JR NZ - Jump relative if not zero (Z flag is not set)
pub fn jr_nz(state: &mut State) {
    let offset = state.read(state.pc) as i8;
    state.pc += 1;

    if !state.flag_z() {
        state.pc = state.pc.wrapping_add(offset as u16);
    }
}

/// DAA - Decimal Adjust Accumulator
/// Adjusts the accumulator for BCD (Binary Coded Decimal) arithmetic
/// after addition or subtraction operations
/// see: https://blog.ollien.com/posts/gb-daa/
pub fn daa(state: &mut State) {
    let mut a = state.a;
    let mut adjust = 0u8;

    if state.flag_h() || (!state.flag_n() && (a & 0x0F) > 0x09) {
        adjust |= 0x06;
    }

    if state.flag_c() || (!state.flag_n() && a > 0x99) {
        adjust |= 0x60;
        state.set_flag_c(true);
    }

    if state.flag_n() {
        a = a.wrapping_sub(adjust);
    } else {
        a = a.wrapping_add(adjust);
    }

    state.a = a;
    state.set_flag_z(a == 0);
    state.set_flag_h(false);
}

/// JR Z - Jump relative if zero (Z flag is set)
pub fn jr_z(state: &mut State) {
    let offset = state.read(state.pc) as i8;
    state.pc += 1;

    if state.flag_z() {
        state.pc = state.pc.wrapping_add(offset as u16);
    }
}

/// JR NC - Jump relative if not carry (C flag is not set)
pub fn jr_nc(state: &mut State) {
    let offset = state.read(state.pc) as i8;
    state.pc += 1;

    if !state.flag_c() {
        state.pc = state.pc.wrapping_add(offset as u16);
    }
}

/// JR C - Jump relative if carry (C flag is set)
pub fn jr_c(state: &mut State) {
    let offset = state.read(state.pc) as i8;
    state.pc += 1;

    if state.flag_c() {
        state.pc = state.pc.wrapping_add(offset as u16);
    }
}

/// CPL - Complement accumulator (flip all bits)
pub fn cpl(state: &mut State) {
    state.a = !state.a;
    state.set_flag_n(true);
    state.set_flag_h(true);
}

/// SCF - Set Carry Flag
pub fn scf(state: &mut State) {
    state.set_flag_c(true);
    state.set_flag_n(false);
    state.set_flag_h(false);
}

/// CCF - Complement Carry Flag
pub fn ccf(state: &mut State) {
    state.set_flag_c(!state.flag_c());
    state.set_flag_n(false);
    state.set_flag_h(false);
}

/// INC (HL) - Increment value at memory location pointed to by HL
pub fn inc_hl_indirect(state: &mut State) {
    let addr = state.hl();
    let value = state.read(addr);
    let result = inc_8bit(value, state);
    state.write(addr, result);
}

/// DEC (HL) - Decrement value at memory location pointed to by HL
pub fn dec_hl_indirect(state: &mut State) {
    let addr = state.hl();
    let value = state.read(addr);
    let result = dec_8bit(value, state);
    state.write(addr, result);
}

/// Add 16-bit value to HL and update flags
/// N flag is reset, H flag is set on carry from bit 11, C flag is set on carry from bit 15
/// Z flag is not affected
fn add_hl(value: u16, state: &mut State) {
    let hl = state.hl();
    let result = hl.wrapping_add(value);

    state.set_flag_n(false);
    // Half carry: check if there's a carry from bit 11 to bit 12
    state.set_flag_h((hl & 0x0FFF) + (value & 0x0FFF) > 0x0FFF);
    // Carry: check if there's a carry from bit 15
    state.set_flag_c(hl > 0xFFFF - value);

    state.set_hl(result);
}

/// ADD HL,BC - Add BC to HL
pub fn add_hl_bc(state: &mut State) {
    let bc = state.bc();
    add_hl(bc, state);
}

/// ADD HL,DE - Add DE to HL
pub fn add_hl_de(state: &mut State) {
    let de = state.de();
    add_hl(de, state);
}

/// ADD HL,HL - Add HL to HL (double HL)
pub fn add_hl_hl(state: &mut State) {
    let hl = state.hl();
    add_hl(hl, state);
}

/// ADD HL,SP - Add SP to HL
pub fn add_hl_sp(state: &mut State) {
    let sp = state.sp();
    add_hl(sp, state);
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
    // Service any pending interrupts
    if service_interrupts(state) {
        // Interrupt was serviced, return early (PC now points to interrupt handler)
        return;
    }

    // Handle delayed interrupt enable/disable (EI and DI take effect after next instruction)
    // This must happen before halt check so IME changes are processed even when halted
    if state.di_delay {
        let prev_opcode = state.read(state.pc.wrapping_sub(1));
        if prev_opcode != 0xF3 {
            // 0xF3 is DI opcode
            state.di_delay = false;
            state.ime = false;
        }
    }

    if state.ei_delay {
        let prev_opcode = state.read(state.pc.wrapping_sub(1));
        if prev_opcode != 0xFB {
            // 0xFB is EI opcode
            state.ei_delay = false;
            state.ime = true;
        }
    }

    // If halted, check for interrupts to wake up
    if state.halt {
        // Check if there are any pending interrupts
        if has_pending_interrupt(state) {
            state.halt = false;
            // If IME is enabled, the interrupt will be handled by interrupt logic
            // If IME is disabled, we just continue execution (HALT bug behavior)
        } else {
            // Still halted, don't execute instruction
            return;
        }
    }

    // TODO: This is not fully correct, in fact the read function must take into consideration the
    // current emomory bank and other detalis.
    let op = state.read(state.pc);
    state.pc += 1;

    match op {
        0x00 => {
            /* NOP */
            state.cycles += 4;
        }
        0x01 => {
            /* LD BC,n */
            state.set_bc(state.read_word(state.pc));
            state.pc += 2;
            state.cycles += 12;
        }
        0x02 => {
            /* LD (BC),A */
            state.write(state.bc(), state.a);
            state.cycles += 8;
        }
        0x03 => {
            /* INC BC */
            inc_bc(state);
            state.cycles += 8;
        }
        0x04 => {
            /* INC B */
            inc_b(state);
            state.cycles += 4;
        }
        0x05 => {
            /* DEC B */
            dec_b(state);
            state.cycles += 4;
        }
        0x06 => {
            /* LD B,n */
            state.b = state.read(state.pc);
            state.pc += 1;
            state.cycles += 8;
        }
        0x07 => {
            /* RLCA */
            rlca(state);
            state.cycles += 4;
        }
        0x08 => {
            /* LD (n),SP */
            // TODO: Verify write_word exists, then implement
            unimplemented!("LD (n),SP - needs write_word function");
        }
        0x09 => {
            /* ADD HL,BC */
            add_hl_bc(state);
            state.cycles += 8;
        }
        0x0A => {
            /* LD A,(BC) */
            state.a = state.read(state.bc());
            state.cycles += 8;
        }
        0x0B => {
            /* DEC BC */
            state.set_bc(state.bc().wrapping_sub(1));
            state.cycles += 8;
        }
        0x0C => {
            /* INC C */
            inc_c(state);
            state.cycles += 4;
        }
        0x0D => {
            /* DEC C */
            dec_c(state);
            state.cycles += 4;
        }
        0x0E => {
            /* LD C,n */
            state.c = state.read(state.pc);
            state.pc += 1;
            state.cycles += 8;
        }
        0x0F => {
            /* RRCA */
            rrca(state);
            state.cycles += 4;
        }
        0x10 => {
            /* STOP */
            state.pc += 1;
            state.cycles += 4;
        }
        0x11 => {
            /* LD DE,n */
            state.set_de(state.read_word(state.pc));
            state.pc += 2;
            state.cycles += 12;
        }
        0x12 => {
            /* LD (DE),A */
            state.write(state.de(), state.a);
            state.cycles += 8;
        }
        0x13 => {
            /* INC DE */
            inc_de(state);
            state.cycles += 8;
        }
        0x14 => {
            /* INC D */
            inc_d(state);
            state.cycles += 4;
        }
        0x15 => {
            /* DEC D */
            dec_d(state);
            state.cycles += 4;
        }
        0x16 => {
            /* LD D,n */
            state.d = state.read(state.pc);
            state.pc += 1;
            state.cycles += 8;
        }
        0x17 => {
            /* RLA */
            rla(state);
            state.cycles += 4;
        }
        0x18 => {
            /* JR */
            jr(state);
            state.cycles += 8;
        }
        0x19 => {
            /* ADD HL,DE */
            add_hl_de(state);
            state.cycles += 8;
        }
        0x1A => {
            /* LD A,(DE) */
            state.a = state.read(state.de());
            state.cycles += 8;
        }
        0x1B => {
            /* DEC DE */
            dec_de(state);
            state.cycles += 8;
        }
        0x1C => {
            /* INC E */
            inc_e(state);
            state.cycles += 4;
        }
        0x1D => {
            /* DEC E */
            dec_e(state);
            state.cycles += 4;
        }
        0x1E => {
            /* LD E,n */
            state.e = state.read(state.pc);
            state.pc += 1;
            state.cycles += 8;
        }
        0x1F => {
            /* RRA */
            rra(state);
            state.cycles += 4;
        }
        0x20 => {
            /* JR NZ */
            jr_nz(state);
            state.cycles += 8;
        }
        0x21 => {
            /* LD HL,n */
            state.set_hl(state.read_word(state.pc));
            state.pc += 2;
            state.cycles += 12;
        }
        0x22 => {
            /* LDI (HL),A */
            state.write(state.hl(), state.a);
            state.set_hl(state.hl().wrapping_add(1));
            state.cycles += 8;
        }
        0x23 => {
            /* INC HL */
            inc_hl(state);
            state.cycles += 8;
        }
        0x24 => {
            /* INC H */
            inc_h(state);
            state.cycles += 4;
        }
        0x25 => {
            /* DEC H */
            dec_h(state);
            state.cycles += 4;
        }
        0x26 => {
            /* LD H,n */
            state.h = state.read(state.pc);
            state.pc += 1;
            state.cycles += 8;
        }
        0x27 => {
            /* DAA */
            daa(state);
            state.cycles += 4;
        }
        0x28 => {
            /* JR Z */
            jr_z(state);
            state.cycles += 8;
        }
        0x29 => {
            /* ADD HL,HL */
            add_hl_hl(state);
            state.cycles += 8;
        }
        0x2A => {
            /* LDI A,(HL) */
            state.a = state.read(state.hl());
            state.set_hl(state.hl().wrapping_add(1));
            state.cycles += 8;
        }
        0x2B => {
            /* DEC HL */
            dec_hl(state);
            state.cycles += 8;
        }
        0x2C => {
            /* INC L */
            inc_l(state);
            state.cycles += 4;
        }
        0x2D => {
            /* DEC L */
            dec_l(state);
            state.cycles += 4;
        }
        0x2E => {
            /* LD L,n */
            state.l = state.read(state.pc);
            state.pc += 1;
            state.cycles += 8;
        }
        0x2F => {
            /* CPL */
            cpl(state);
            state.cycles += 4;
        }
        0x30 => {
            /* JR NC */
            jr_nc(state);
            state.cycles += 8;
        }
        0x31 => {
            /* LD SP,n */
            state.set_sp(state.read_word(state.pc));
            state.pc += 2;
            state.cycles += 12;
        }
        0x32 => {
            /* LDD (HL),A */
            state.write(state.hl(), state.a);
            state.set_hl(state.hl().wrapping_sub(1));
            state.cycles += 8;
        }
        0x33 => {
            /* INC SP */
            inc_sp(state);
            state.cycles += 8;
        }
        0x34 => {
            /* INC (HL) */
            inc_hl_indirect(state);
            state.cycles += 12;
        }
        0x35 => {
            /* DEC (HL) */
            dec_hl_indirect(state);
            state.cycles += 12;
        }
        0x36 => {
            /* LD (HL),n */
            let value = state.read(state.pc);
            state.pc += 1;
            state.write(state.hl(), value);
            state.cycles += 12;
        }
        0x37 => {
            /* SCF */
            scf(state);
            state.cycles += 4;
        }
        0x38 => {
            /* JR C */
            jr_c(state);
            state.cycles += 8;
        }
        0x39 => {
            /* ADD HL,SP */
            add_hl_sp(state);
            state.cycles += 8;
        }
        0x3A => {
            /* LDD A,(HL) */
            state.a = state.read(state.hl());
            state.set_hl(state.hl().wrapping_sub(1));
            state.cycles += 8;
        }
        0x3B => {
            /* DEC SP */
            dec_sp(state);
            state.cycles += 8;
        }
        0x3C => {
            /* INC A */
            inc_a(state);
            state.cycles += 4;
        }
        0x3D => {
            /* DEC A */
            dec_a(state);
            state.cycles += 4;
        }
        0x3E => {
            /* LD A,n */
            state.a = state.read(state.pc);
            state.cycles += 8;
            state.pc += 1;
        }
        0x3F => {
            /* CCF */
            ccf(state);
            state.cycles += 4;
        }
        0x40 => {
            /* LD B,B */
            state.cycles += 4;
        }
        0x41 => {
            /* LD B,C */
            state.b = state.c;
            state.cycles += 4;
        }
        0x42 => {
            /* LD B,D */
            state.b = state.d;
            state.cycles += 4;
        }
        0x43 => {
            /* LD B,E */
            state.b = state.e;
            state.cycles += 4;
        }
        0x44 => {
            /* LD B,H */
            state.b = state.h;
            state.cycles += 4;
        }
        0x45 => {
            /* LD B,L */
            state.b = state.l;
            state.cycles += 4;
        }
        0x46 => {
            /* LD B,(HL) */
            state.b = state.read(state.hl());
            state.cycles += 8;
        }
        0x47 => {
            /* LD B,A */
            state.b = state.a;
            state.cycles += 4;
        }
        0x48 => {
            /* LD C,B */
            state.c = state.b;
            state.cycles += 4;
        }
        0x49 => {
            /* LD C,C */
            state.cycles += 4;
        }
        0x4A => {
            /* LD C,D */
            state.c = state.d;
            state.cycles += 4;
        }
        0x4B => {
            /* LD C,E */
            state.c = state.e;
            state.cycles += 4;
        }
        0x4C => {
            /* LD C,H */
            state.c = state.h;
            state.cycles += 4;
        }
        0x4D => {
            /* LD C,L */
            state.c = state.l;
            state.cycles += 4;
        }
        0x4E => {
            /* LD C,(HL) */
            state.c = state.read(state.hl());
            state.cycles += 8;
        }
        0x4F => {
            /* LD C,A */
            state.c = state.a;
            state.cycles += 4;
        }
        0x50 => {
            /* LD D,B */
            state.d = state.b;
            state.cycles += 4;
        }
        0x51 => {
            /* LD D,C */
            state.d = state.c;
            state.cycles += 4;
        }
        0x52 => {
            /* LD D,D */
            state.cycles += 4;
        }
        0x53 => {
            /* LD D,E */
            state.d = state.e;
            state.cycles += 4;
        }
        0x54 => {
            /* LD D,H */
            state.d = state.h;
            state.cycles += 4;
        }
        0x55 => {
            /* LD D,L */
            state.d = state.l;
            state.cycles += 4;
        }
        0x56 => {
            /* LD D,(HL) */
            state.d = state.read(state.hl());
            state.cycles += 8;
        }
        0x57 => {
            /* LD D,A */
            state.d = state.a;
            state.cycles += 4;
        }
        0x58 => {
            /* LD E,B */
            state.e = state.b;
            state.cycles += 4;
        }
        0x59 => {
            /* LD E,C */
            state.e = state.c;
            state.cycles += 4;
        }
        0x5A => {
            /* LD E,D */
            state.e = state.d;
            state.cycles += 4;
        }
        0x5B => {
            /* LD E,E */
            state.cycles += 4;
        }
        0x5C => {
            /* LD E,H */
            state.e = state.h;
            state.cycles += 4;
        }
        0x5D => {
            /* LD E,L */
            state.e = state.l;
            state.cycles += 4;
        }
        0x5E => {
            /* LD E,(HL) */
            state.e = state.read(state.hl());
            state.cycles += 8;
        }
        0x5F => {
            /* LD E,A */
            state.e = state.a;
            state.cycles += 4;
        }
        0x60 => {
            /* LD H,B */
            state.h = state.b;
            state.cycles += 4;
        }
        0x61 => {
            /* LD H,C */
            state.h = state.c;
            state.cycles += 4;
        }
        0x62 => {
            /* LD H,D */
            state.h = state.d;
            state.cycles += 4;
        }
        0x63 => {
            /* LD H,E */
            state.h = state.e;
            state.cycles += 4;
        }
        0x64 => {
            /* LD H,H */
            state.cycles += 4;
        }
        0x65 => {
            /* LD H,L */
            state.h = state.l;
            state.cycles += 4;
        }
        0x66 => {
            /* LD H,(HL) */
            state.h = state.read(state.hl());
            state.cycles += 8;
        }
        0x67 => {
            /* LD H,A */
            state.h = state.a;
            state.cycles += 4;
        }
        0x68 => {
            /* LD L,B */
            state.l = state.b;
            state.cycles += 4;
        }
        0x69 => {
            /* LD L,C */
            state.l = state.c;
            state.cycles += 4;
        }
        0x6A => {
            /* LD L,D */
            state.l = state.d;
            state.cycles += 4;
        }
        0x6B => {
            /* LD L,E */
            state.l = state.e;
            state.cycles += 4;
        }
        0x6C => {
            /* LD L,H */
            state.l = state.h;
            state.cycles += 4;
        }
        0x6D => {
            /* LD L,L */
            state.cycles += 4;
        }
        0x6E => {
            /* LD L,(HL) */
            state.l = state.read(state.hl());
            state.cycles += 8;
        }
        0x6F => {
            /* LD L,A */
            state.l = state.a;
            state.cycles += 4;
        }
        0x70 => {
            /* LD (HL),B */
            state.write(state.hl(), state.b);
            state.cycles += 8;
        }
        0x71 => {
            /* LD (HL),C */
            state.write(state.hl(), state.c);
            state.cycles += 8;
        }
        0x72 => {
            /* LD (HL),D */
            state.write(state.hl(), state.d);
            state.cycles += 8;
        }
        0x73 => {
            /* LD (HL),E */
            state.write(state.hl(), state.e);
            state.cycles += 8;
        }
        0x74 => {
            /* LD (HL),H */
            state.write(state.hl(), state.h);
            state.cycles += 8;
        }
        0x75 => {
            /* LD (HL),L */
            state.write(state.hl(), state.l);
            state.cycles += 8;
        }
        0x76 => {
            /* HALT */
            if state.ime {
                state.halt = true;
            }
            state.cycles += 4;
        }
        0x77 => {
            /* LD (HL),A */
            state.write(state.hl(), state.a);
            state.cycles += 8;
        }
        0x78 => {
            /* LD A,B */
            state.a = state.b;
            state.cycles += 4;
        }
        0x79 => {
            /* LD A,C */
            state.a = state.c;
            state.cycles += 4;
        }
        0x7A => {
            /* LD A,D */
            state.a = state.d;
            state.cycles += 4;
        }
        0x7B => {
            /* LD A,E */
            state.a = state.e;
            state.cycles += 4;
        }
        0x7C => {
            /* LD A,H */
            state.a = state.h;
            state.cycles += 4;
        }
        0x7D => {
            /* LD A,L */
            state.a = state.l;
            state.cycles += 4;
        }
        0x7E => {
            /* LD A,(HL) */
            state.a = state.read(state.hl());
            state.cycles += 8;
        }
        0x7F => {
            /* LD A,A */
            // No-op, but still takes cycles
            state.cycles += 4;
        }
        0x80 => {
            /* ADD A,B */
            add_a(state.b, state);
            state.cycles += 4;
        }
        0x81 => {
            /* ADD A,C */
            add_a(state.c, state);
            state.cycles += 4;
        }
        0x82 => {
            /* ADD A,D */
            add_a(state.d, state);
            state.cycles += 4;
        }
        0x83 => {
            /* ADD A,E */
            add_a(state.e, state);
            state.cycles += 4;
        }
        0x84 => {
            /* ADD A,H */
            add_a(state.h, state);
            state.cycles += 4;
        }
        0x85 => {
            /* ADD A,L */
            add_a(state.l, state);
            state.cycles += 4;
        }
        0x86 => {
            /* ADD A,(HL) */
            let value = state.read(state.hl());
            add_a(value, state);
            state.cycles += 8;
        }
        0x87 => {
            /* ADD A,A */
            add_a(state.a, state);
            state.cycles += 4;
        }
        0x88 => {
            /* ADC A,B */
            adc_a(state.b, state);
            state.cycles += 4;
        }
        0x89 => {
            /* ADC A,C */
            adc_a(state.c, state);
            state.cycles += 4;
        }
        0x8A => {
            /* ADC A,D */
            adc_a(state.d, state);
            state.cycles += 4;
        }
        0x8B => {
            /* ADC A,E */
            adc_a(state.e, state);
            state.cycles += 4;
        }
        0x8C => {
            /* ADC A,H */
            adc_a(state.h, state);
            state.cycles += 4;
        }
        0x8D => {
            /* ADC A,L */
            adc_a(state.l, state);
            state.cycles += 4;
        }
        0x8E => {
            /* ADC A,(HL) */
            let value = state.read(state.hl());
            adc_a(value, state);
            state.cycles += 8;
        }
        0x8F => {
            /* ADC A,A */
            adc_a(state.a, state);
            state.cycles += 4;
        }
        0x90 => {
            /* SUB B */
            sub_a(state.b, state);
            state.cycles += 4;
        }
        0x91 => {
            /* SUB C */
            sub_a(state.c, state);
            state.cycles += 4;
        }
        0x92 => {
            /* SUB D */
            sub_a(state.d, state);
            state.cycles += 4;
        }
        0x93 => {
            /* SUB E */
            sub_a(state.e, state);
            state.cycles += 4;
        }
        0x94 => {
            /* SUB H */
            sub_a(state.h, state);
            state.cycles += 4;
        }
        0x95 => {
            /* SUB L */
            sub_a(state.l, state);
            state.cycles += 4;
        }
        0x96 => {
            /* SUB (HL) */
            let value = state.read(state.hl());
            sub_a(value, state);
            state.cycles += 8;
        }
        0x97 => {
            /* SUB A */
            sub_a(state.a, state);
            state.cycles += 4;
        }
        0x98 => {
            /* SBC A,B */
            sbc_a(state.b, state);
            state.cycles += 4;
        }
        0x99 => {
            /* SBC A,C */
            sbc_a(state.c, state);
            state.cycles += 4;
        }
        0x9A => {
            /* SBC A,D */
            sbc_a(state.d, state);
            state.cycles += 4;
        }
        0x9B => {
            /* SBC A,E */
            sbc_a(state.e, state);
            state.cycles += 4;
        }
        0x9C => {
            /* SBC A,H */
            sbc_a(state.h, state);
            state.cycles += 4;
        }
        0x9D => {
            /* SBC A,L */
            sbc_a(state.l, state);
            state.cycles += 4;
        }
        0x9E => {
            /* SBC A,(HL) */
            let value = state.read(state.hl());
            sbc_a(value, state);
            state.cycles += 8;
        }
        0x9F => {
            /* SBC A,A */
            sbc_a(state.a, state);
            state.cycles += 4;
        }
        _ => {
            panic!("Unimplemented opcode: 0x{:02X}", op);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for ADD A,r
    #[test]
    fn test_add_a_normal() {
        let mut state = State::new();
        state.a = 0x3A;

        add_a(0x05, &mut state);

        assert_eq!(state.a, 0x3F);
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_add_a_zero_result() {
        let mut state = State::new();
        state.a = 0x00;

        add_a(0x00, &mut state);

        assert_eq!(state.a, 0x00);
        assert!(state.flag_z()); // Result is zero
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_add_a_carry() {
        let mut state = State::new();
        state.a = 0xFF;

        add_a(0x02, &mut state);

        assert_eq!(state.a, 0x01);
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(state.flag_h()); // Carry from bit 3
        assert!(state.flag_c()); // Carry from bit 7
    }

    #[test]
    fn test_add_a_half_carry() {
        let mut state = State::new();
        state.a = 0x0F;

        add_a(0x01, &mut state);

        assert_eq!(state.a, 0x10);
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(state.flag_h()); // Half carry from bit 3
        assert!(!state.flag_c());
    }

    #[test]
    fn test_add_a_overflow_to_zero() {
        let mut state = State::new();
        state.a = 0xFF;

        add_a(0x01, &mut state);

        assert_eq!(state.a, 0x00);
        assert!(state.flag_z()); // Result is zero
        assert!(!state.flag_n());
        assert!(state.flag_h()); // Half carry
        assert!(state.flag_c()); // Carry
    }

    #[test]
    fn test_add_a_both_carries() {
        let mut state = State::new();
        state.a = 0xFF;

        add_a(0xFF, &mut state);

        assert_eq!(state.a, 0xFE);
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(state.flag_h()); // Half carry
        assert!(state.flag_c()); // Carry
    }

    #[test]
    fn test_add_a_no_half_carry_boundary() {
        let mut state = State::new();
        state.a = 0x0E;

        add_a(0x01, &mut state);

        assert_eq!(state.a, 0x0F);
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h()); // No half carry
        assert!(!state.flag_c());
    }

    // Tests for ADC A,r
    #[test]
    fn test_adc_a_normal_no_carry() {
        let mut state = State::new();
        state.a = 0x3A;
        state.set_flag_c(false);

        adc_a(0x05, &mut state);

        assert_eq!(state.a, 0x3F);
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_adc_a_with_carry_flag() {
        let mut state = State::new();
        state.a = 0x3A;
        state.set_flag_c(true);

        adc_a(0x05, &mut state);

        assert_eq!(state.a, 0x40); // 0x3A + 0x05 + 1 = 0x40
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(state.flag_h()); // 0xA + 0x5 + 1 = 0x10 > 0xF (half carry)
        assert!(!state.flag_c());
    }

    #[test]
    fn test_adc_a_carry_flag_causes_overflow() {
        let mut state = State::new();
        state.a = 0xFF;
        state.set_flag_c(true);

        adc_a(0x00, &mut state);

        assert_eq!(state.a, 0x00); // 0xFF + 0 + 1 = 0x00 (overflow)
        assert!(state.flag_z());
        assert!(!state.flag_n());
        assert!(state.flag_h()); // Half carry
        assert!(state.flag_c()); // Carry
    }

    #[test]
    fn test_adc_a_half_carry_with_carry_flag() {
        let mut state = State::new();
        state.a = 0x0E;
        state.set_flag_c(true);

        adc_a(0x00, &mut state);

        assert_eq!(state.a, 0x0F); // 0x0E + 0 + 1 = 0x0F
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h()); // No half carry
        assert!(!state.flag_c());
    }

    #[test]
    fn test_adc_a_half_carry_from_value_and_carry() {
        let mut state = State::new();
        state.a = 0x0E;
        state.set_flag_c(true);

        adc_a(0x01, &mut state);

        assert_eq!(state.a, 0x10); // 0x0E + 0x01 + 1 = 0x10
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(state.flag_h()); // Half carry
        assert!(!state.flag_c());
    }

    #[test]
    fn test_adc_a_overflow_with_carry() {
        let mut state = State::new();
        state.a = 0xFE;
        state.set_flag_c(true);

        adc_a(0x01, &mut state);

        assert_eq!(state.a, 0x00); // 0xFE + 0x01 + 1 = 0x00
        assert!(state.flag_z());
        assert!(!state.flag_n());
        assert!(state.flag_h()); // Half carry
        assert!(state.flag_c()); // Carry
    }

    #[test]
    fn test_adc_a_both_carries_with_carry_flag() {
        let mut state = State::new();
        state.a = 0xFF;
        state.set_flag_c(true);

        adc_a(0xFF, &mut state);

        assert_eq!(state.a, 0xFF); // 0xFF + 0xFF + 1 = 0xFF (with overflow)
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(state.flag_h()); // Half carry
        assert!(state.flag_c()); // Carry
    }

    // Tests for SUB A,r
    #[test]
    fn test_sub_a_normal() {
        let mut state = State::new();
        state.a = 0x3E;

        sub_a(0x0F, &mut state);

        assert_eq!(state.a, 0x2F);
        assert!(!state.flag_z());
        assert!(state.flag_n()); // Subtraction
        assert!(state.flag_h()); // Borrow: 0xE < 0xF
        assert!(!state.flag_c());
    }

    #[test]
    fn test_sub_a_zero_result() {
        let mut state = State::new();
        state.a = 0x42;

        sub_a(0x42, &mut state);

        assert_eq!(state.a, 0x00);
        assert!(state.flag_z());
        assert!(state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_sub_a_underflow() {
        let mut state = State::new();
        state.a = 0x00;

        sub_a(0x01, &mut state);

        assert_eq!(state.a, 0xFF);
        assert!(!state.flag_z());
        assert!(state.flag_n());
        assert!(state.flag_h()); // Borrow
        assert!(state.flag_c()); // Borrow
    }

    #[test]
    fn test_sub_a_half_borrow() {
        let mut state = State::new();
        state.a = 0x10;

        sub_a(0x01, &mut state);

        assert_eq!(state.a, 0x0F);
        assert!(!state.flag_z());
        assert!(state.flag_n());
        assert!(state.flag_h()); // 0x0 < 0x1 (half borrow)
        assert!(!state.flag_c());
    }

    #[test]
    fn test_sub_a_no_half_borrow() {
        let mut state = State::new();
        state.a = 0x0F;

        sub_a(0x01, &mut state);

        assert_eq!(state.a, 0x0E);
        assert!(!state.flag_z());
        assert!(state.flag_n());
        assert!(!state.flag_h()); // No half borrow
        assert!(!state.flag_c());
    }

    // Tests for SBC A,r
    #[test]
    fn test_sbc_a_normal_no_carry() {
        let mut state = State::new();
        state.a = 0x3E;
        state.set_flag_c(false);

        sbc_a(0x0F, &mut state);

        assert_eq!(state.a, 0x2F);
        assert!(!state.flag_z());
        assert!(state.flag_n());
        assert!(state.flag_h()); // Borrow
        assert!(!state.flag_c());
    }

    #[test]
    fn test_sbc_a_with_carry_flag() {
        let mut state = State::new();
        state.a = 0x3E;
        state.set_flag_c(true);

        sbc_a(0x0F, &mut state);

        assert_eq!(state.a, 0x2E); // 0x3E - 0x0F - 1 = 0x2E
        assert!(!state.flag_z());
        assert!(state.flag_n());
        assert!(state.flag_h()); // 0xE < 0xF + 1
        assert!(!state.flag_c());
    }

    #[test]
    fn test_sbc_a_zero_result_with_carry() {
        let mut state = State::new();
        state.a = 0x01;
        state.set_flag_c(true);

        sbc_a(0x00, &mut state);

        assert_eq!(state.a, 0x00); // 0x01 - 0x00 - 1 = 0x00
        assert!(state.flag_z());
        assert!(state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_sbc_a_underflow_with_carry() {
        let mut state = State::new();
        state.a = 0x00;
        state.set_flag_c(true);

        sbc_a(0x00, &mut state);

        assert_eq!(state.a, 0xFF); // 0x00 - 0x00 - 1 = 0xFF
        assert!(!state.flag_z());
        assert!(state.flag_n());
        assert!(state.flag_h()); // Borrow
        assert!(state.flag_c()); // Borrow
    }

    #[test]
    fn test_sbc_a_half_borrow_from_carry() {
        let mut state = State::new();
        state.a = 0x10;
        state.set_flag_c(true);

        sbc_a(0x00, &mut state);

        assert_eq!(state.a, 0x0F); // 0x10 - 0x00 - 1 = 0x0F
        assert!(!state.flag_z());
        assert!(state.flag_n());
        assert!(state.flag_h()); // 0x0 < 0x0 + 1
        assert!(!state.flag_c());
    }

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

    #[test]
    fn test_rlc_normal() {
        let mut state = State::new();
        state.a = 0b0100_1010; // 0x4A

        rlc_a(&mut state);

        assert_eq!(state.a, 0b1001_0100); // 0x94
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c()); // Bit 7 was 0
    }

    #[test]
    fn test_rlc_with_carry() {
        let mut state = State::new();
        state.b = 0b1100_1010; // 0xCA

        rlc_b(&mut state);

        assert_eq!(state.b, 0b1001_0101); // 0x95
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 7 was 1
    }

    #[test]
    fn test_rlc_zero_result() {
        let mut state = State::new();
        state.c = 0x00;

        rlc_c(&mut state);

        assert_eq!(state.c, 0x00);
        assert!(state.flag_z()); // Result is zero
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_rlc_bit7_wraps() {
        let mut state = State::new();
        state.d = 0x80; // 0b1000_0000

        rlc_d(&mut state);

        assert_eq!(state.d, 0x01); // 0b0000_0001 - bit 7 wraps to bit 0
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 7 was 1
    }

    #[test]
    fn test_rlc_all_registers() {
        let mut state = State::new();

        state.a = 0x01;
        rlc_a(&mut state);
        assert_eq!(state.a, 0x02);

        state.b = 0x01;
        rlc_b(&mut state);
        assert_eq!(state.b, 0x02);

        state.c = 0x01;
        rlc_c(&mut state);
        assert_eq!(state.c, 0x02);

        state.d = 0x01;
        rlc_d(&mut state);
        assert_eq!(state.d, 0x02);

        state.e = 0x01;
        rlc_e(&mut state);
        assert_eq!(state.e, 0x02);

        state.h = 0x01;
        rlc_h(&mut state);
        assert_eq!(state.h, 0x02);

        state.l = 0x01;
        rlc_l(&mut state);
        assert_eq!(state.l, 0x02);
    }

    #[test]
    fn test_rlc_all_bits() {
        let mut state = State::new();
        state.a = 0xFF;

        rlc_a(&mut state);

        assert_eq!(state.a, 0xFF); // All bits rotate, stays same
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 7 was 1
    }

    #[test]
    fn test_rlca_always_resets_z() {
        let mut state = State::new();
        state.a = 0x00;

        rlca(&mut state);

        assert_eq!(state.a, 0x00);
        assert!(!state.flag_z()); // RLCA always resets Z, even when result is 0
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_rlca_normal() {
        let mut state = State::new();
        state.a = 0b1100_1010; // 0xCA

        rlca(&mut state);

        assert_eq!(state.a, 0b1001_0101); // 0x95
        assert!(!state.flag_z()); // RLCA always resets Z
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 7 was 1
    }

    #[test]
    fn test_rrc_normal() {
        let mut state = State::new();
        state.a = 0b0100_1010; // 0x4A

        rrc_a(&mut state);

        assert_eq!(state.a, 0b0010_0101); // 0x25
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c()); // Bit 0 was 0
    }

    #[test]
    fn test_rrc_with_carry() {
        let mut state = State::new();
        state.b = 0b1100_1011; // 0xCB

        rrc_b(&mut state);

        assert_eq!(state.b, 0b1110_0101); // 0xE5
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 0 was 1
    }

    #[test]
    fn test_rrc_zero_result() {
        let mut state = State::new();
        state.c = 0x00;

        rrc_c(&mut state);

        assert_eq!(state.c, 0x00);
        assert!(state.flag_z()); // Result is zero
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_rrc_bit0_wraps() {
        let mut state = State::new();
        state.d = 0x01; // 0b0000_0001

        rrc_d(&mut state);

        assert_eq!(state.d, 0x80); // 0b1000_0000 - bit 0 wraps to bit 7
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 0 was 1
    }

    #[test]
    fn test_rrc_all_registers() {
        let mut state = State::new();

        state.a = 0x80;
        rrc_a(&mut state);
        assert_eq!(state.a, 0x40);

        state.b = 0x80;
        rrc_b(&mut state);
        assert_eq!(state.b, 0x40);

        state.c = 0x80;
        rrc_c(&mut state);
        assert_eq!(state.c, 0x40);

        state.d = 0x80;
        rrc_d(&mut state);
        assert_eq!(state.d, 0x40);

        state.e = 0x80;
        rrc_e(&mut state);
        assert_eq!(state.e, 0x40);

        state.h = 0x80;
        rrc_h(&mut state);
        assert_eq!(state.h, 0x40);

        state.l = 0x80;
        rrc_l(&mut state);
        assert_eq!(state.l, 0x40);
    }

    #[test]
    fn test_rrc_all_bits() {
        let mut state = State::new();
        state.a = 0xFF;

        rrc_a(&mut state);

        assert_eq!(state.a, 0xFF); // All bits rotate, stays same
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 0 was 1
    }

    #[test]
    fn test_rrca_always_resets_z() {
        let mut state = State::new();
        state.a = 0x00;

        rrca(&mut state);

        assert_eq!(state.a, 0x00);
        assert!(!state.flag_z()); // RRCA always resets Z, even when result is 0
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_rrca_normal() {
        let mut state = State::new();
        state.a = 0b1100_1011; // 0xCB

        rrca(&mut state);

        assert_eq!(state.a, 0b1110_0101); // 0xE5
        assert!(!state.flag_z()); // RRCA always resets Z
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 0 was 1
    }

    #[test]
    fn test_rl_normal_carry_clear() {
        let mut state = State::new();
        state.a = 0b0100_1010; // 0x4A
        state.set_flag_c(false);

        rl_a(&mut state);

        assert_eq!(state.a, 0b1001_0100); // 0x94
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c()); // Bit 7 was 0
    }

    #[test]
    fn test_rl_normal_carry_set() {
        let mut state = State::new();
        state.b = 0b0100_1010; // 0x4A
        state.set_flag_c(true);

        rl_b(&mut state);

        assert_eq!(state.b, 0b1001_0101); // 0x95 (carry flag becomes bit 0)
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c()); // Bit 7 was 0
    }

    #[test]
    fn test_rl_with_carry_out() {
        let mut state = State::new();
        state.c = 0b1100_1010; // 0xCA
        state.set_flag_c(false);

        rl_c(&mut state);

        assert_eq!(state.c, 0b1001_0100); // 0x94
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 7 was 1
    }

    #[test]
    fn test_rl_zero_result() {
        let mut state = State::new();
        state.d = 0x00;
        state.set_flag_c(false);

        rl_d(&mut state);

        assert_eq!(state.d, 0x00);
        assert!(state.flag_z()); // Result is zero
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_rl_bit7_to_carry() {
        let mut state = State::new();
        state.e = 0x80; // 0b1000_0000
        state.set_flag_c(true);

        rl_e(&mut state);

        assert_eq!(state.e, 0x01); // 0b0000_0001 (carry in becomes bit 0)
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 7 was 1
    }

    #[test]
    fn test_rl_all_registers() {
        let mut state = State::new();

        state.a = 0x01;
        state.set_flag_c(false);
        rl_a(&mut state);
        assert_eq!(state.a, 0x02);

        state.b = 0x01;
        state.set_flag_c(false);
        rl_b(&mut state);
        assert_eq!(state.b, 0x02);

        state.c = 0x01;
        state.set_flag_c(false);
        rl_c(&mut state);
        assert_eq!(state.c, 0x02);

        state.d = 0x01;
        state.set_flag_c(false);
        rl_d(&mut state);
        assert_eq!(state.d, 0x02);

        state.e = 0x01;
        state.set_flag_c(false);
        rl_e(&mut state);
        assert_eq!(state.e, 0x02);

        state.h = 0x01;
        state.set_flag_c(false);
        rl_h(&mut state);
        assert_eq!(state.h, 0x02);

        state.l = 0x01;
        state.set_flag_c(false);
        rl_l(&mut state);
        assert_eq!(state.l, 0x02);
    }

    #[test]
    fn test_rl_carry_propagation() {
        let mut state = State::new();
        state.a = 0xFF;
        state.set_flag_c(true);

        rl_a(&mut state);

        assert_eq!(state.a, 0xFF); // All bits set, carry in becomes bit 0
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 7 was 1
    }

    #[test]
    fn test_rla_always_resets_z() {
        let mut state = State::new();
        state.a = 0x00;
        state.set_flag_c(false);

        rla(&mut state);

        assert_eq!(state.a, 0x00);
        assert!(!state.flag_z()); // RLA always resets Z, even when result is 0
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_rla_normal() {
        let mut state = State::new();
        state.a = 0b1100_1010; // 0xCA
        state.set_flag_c(true);

        rla(&mut state);

        assert_eq!(state.a, 0b1001_0101); // 0x95 (carry becomes bit 0)
        assert!(!state.flag_z()); // RLA always resets Z
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 7 was 1
    }

    #[test]
    fn test_rla_without_carry() {
        let mut state = State::new();
        state.a = 0b0100_1010; // 0x4A
        state.set_flag_c(false);

        rla(&mut state);

        assert_eq!(state.a, 0b1001_0100); // 0x94
        assert!(!state.flag_z()); // RLA always resets Z
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c()); // Bit 7 was 0
    }

    #[test]
    fn test_jr_positive_offset() {
        let mut state = State::new();
        state.pc = 0x1000;
        state.write(0x1000, 0x10); // Jump forward by 16 bytes

        jr(&mut state);

        // PC should be 0x1000 + 1 (read offset) + 0x10 (offset) = 0x1011
        assert_eq!(state.pc, 0x1011);
    }

    #[test]
    fn test_jr_negative_offset() {
        let mut state = State::new();
        state.pc = 0x1000;
        state.write(0x1000, 0xFE); // Jump backward by 2 bytes (-2 as signed i8)

        jr(&mut state);

        // PC should be 0x1000 + 1 (read offset) + (-2) = 0x0FFF
        assert_eq!(state.pc, 0x0FFF);
    }

    #[test]
    fn test_jr_zero_offset() {
        let mut state = State::new();
        state.pc = 0x1000;
        state.write(0x1000, 0x00); // No jump, just move to next instruction

        jr(&mut state);

        // PC should be 0x1000 + 1 = 0x1001 (infinite loop at current position)
        assert_eq!(state.pc, 0x1001);
    }

    #[test]
    fn test_jr_max_positive_offset() {
        let mut state = State::new();
        state.pc = 0x1000;
        state.write(0x1000, 0x7F); // Jump forward by 127 bytes (max positive i8)

        jr(&mut state);

        // PC should be 0x1000 + 1 + 0x7F = 0x1080
        assert_eq!(state.pc, 0x1080);
    }

    #[test]
    fn test_jr_max_negative_offset() {
        let mut state = State::new();
        state.pc = 0x1000;
        state.write(0x1000, 0x80); // Jump backward by 128 bytes (min i8 = -128)

        jr(&mut state);

        // PC should be 0x1000 + 1 + (-128) = 0x1001 - 128 = 0x0F81
        assert_eq!(state.pc, 0x0F81);
    }

    #[test]
    fn test_jr_wrap_around() {
        let mut state = State::new();
        state.pc = 0xFFFE;
        state.write(0xFFFE, 0x05); // Jump forward by 5 bytes

        jr(&mut state);

        // PC should wrap around: 0xFFFE + 1 + 5 = 0x10004 -> 0x0004 (wrapping)
        assert_eq!(state.pc, 0x0004);
    }

    #[test]
    fn test_jr_backward_loop() {
        let mut state = State::new();
        state.pc = 0x1005;
        state.write(0x1005, 0xFE); // -2, creates infinite loop

        jr(&mut state);

        // PC should be 0x1005 + 1 + (-2) = 0x1004
        assert_eq!(state.pc, 0x1004);
    }

    #[test]
    fn test_add_hl_bc_normal() {
        let mut state = State::new();
        state.set_hl(0x1000);
        state.set_bc(0x0234);

        add_hl_bc(&mut state);

        assert_eq!(state.hl(), 0x1234);
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_add_hl_de_normal() {
        let mut state = State::new();
        state.set_hl(0x2000);
        state.set_de(0x0500);

        add_hl_de(&mut state);

        assert_eq!(state.hl(), 0x2500);
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_add_hl_half_carry() {
        let mut state = State::new();
        state.set_hl(0x0FFF);
        state.set_bc(0x0001);

        add_hl_bc(&mut state);

        assert_eq!(state.hl(), 0x1000);
        assert!(!state.flag_n());
        assert!(state.flag_h()); // Half carry from bit 11
        assert!(!state.flag_c());
    }

    #[test]
    fn test_add_hl_carry() {
        let mut state = State::new();
        state.set_hl(0xFFFF);
        state.set_de(0x0001);

        add_hl_de(&mut state);

        assert_eq!(state.hl(), 0x0000);
        assert!(!state.flag_n());
        assert!(state.flag_h()); // Half carry from bit 11
        assert!(state.flag_c()); // Carry from bit 15
    }

    #[test]
    fn test_add_hl_both_carries() {
        let mut state = State::new();
        state.set_hl(0xFFFF);
        state.set_bc(0xFFFF);

        add_hl_bc(&mut state);

        assert_eq!(state.hl(), 0xFFFE);
        assert!(!state.flag_n());
        assert!(state.flag_h()); // Half carry from bit 11
        assert!(state.flag_c()); // Carry from bit 15
    }

    #[test]
    fn test_add_hl_hl_double() {
        let mut state = State::new();
        state.set_hl(0x1234);

        add_hl_hl(&mut state);

        assert_eq!(state.hl(), 0x2468);
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_add_hl_sp() {
        let mut state = State::new();
        state.set_hl(0x1000);
        state.set_sp(0x0200);

        add_hl_sp(&mut state);

        assert_eq!(state.hl(), 0x1200);
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_add_hl_z_flag_preserved() {
        let mut state = State::new();
        state.set_hl(0x1000);
        state.set_bc(0x1000);
        state.set_flag_z(true); // Set Z flag

        add_hl_bc(&mut state);

        assert_eq!(state.hl(), 0x2000);
        assert!(state.flag_z()); // Z flag should be preserved
        assert!(!state.flag_n());
    }

    #[test]
    fn test_add_hl_half_carry_boundary() {
        let mut state = State::new();
        state.set_hl(0x0800);
        state.set_de(0x0800);

        add_hl_de(&mut state);

        assert_eq!(state.hl(), 0x1000);
        assert!(!state.flag_n());
        assert!(state.flag_h()); // Half carry at bit 11
        assert!(!state.flag_c());
    }

    #[test]
    fn test_add_hl_no_half_carry_just_below() {
        let mut state = State::new();
        state.set_hl(0x07FF);
        state.set_bc(0x0800);

        add_hl_bc(&mut state);

        assert_eq!(state.hl(), 0x0FFF);
        assert!(!state.flag_n());
        assert!(!state.flag_h()); // No half carry
        assert!(!state.flag_c());
    }

    #[test]
    fn test_add_hl_carry_boundary() {
        let mut state = State::new();
        state.set_hl(0x8000);
        state.set_de(0x8000);

        add_hl_de(&mut state);

        assert_eq!(state.hl(), 0x0000);
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Carry from bit 15
    }

    #[test]
    fn test_rr_normal_carry_clear() {
        let mut state = State::new();
        state.a = 0b1001_0100; // 0x94
        state.set_flag_c(false);

        rr_a(&mut state);

        assert_eq!(state.a, 0b0100_1010); // 0x4A
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c()); // Bit 0 was 0
    }

    #[test]
    fn test_rr_normal_carry_set() {
        let mut state = State::new();
        state.b = 0b1001_0100; // 0x94
        state.set_flag_c(true);

        rr_b(&mut state);

        assert_eq!(state.b, 0b1100_1010); // 0xCA (carry flag becomes bit 7)
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c()); // Bit 0 was 0
    }

    #[test]
    fn test_rr_with_carry_out() {
        let mut state = State::new();
        state.c = 0b1001_0101; // 0x95
        state.set_flag_c(false);

        rr_c(&mut state);

        assert_eq!(state.c, 0b0100_1010); // 0x4A
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 0 was 1
    }

    #[test]
    fn test_rr_zero_result() {
        let mut state = State::new();
        state.d = 0x00;
        state.set_flag_c(false);

        rr_d(&mut state);

        assert_eq!(state.d, 0x00);
        assert!(state.flag_z()); // Result is zero
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_rr_bit0_to_carry() {
        let mut state = State::new();
        state.e = 0x01; // 0b0000_0001
        state.set_flag_c(true);

        rr_e(&mut state);

        assert_eq!(state.e, 0x80); // 0b1000_0000 (carry in becomes bit 7)
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 0 was 1
    }

    #[test]
    fn test_rr_all_registers() {
        let mut state = State::new();

        state.a = 0x80;
        state.set_flag_c(false);
        rr_a(&mut state);
        assert_eq!(state.a, 0x40);

        state.b = 0x80;
        state.set_flag_c(false);
        rr_b(&mut state);
        assert_eq!(state.b, 0x40);

        state.c = 0x80;
        state.set_flag_c(false);
        rr_c(&mut state);
        assert_eq!(state.c, 0x40);

        state.d = 0x80;
        state.set_flag_c(false);
        rr_d(&mut state);
        assert_eq!(state.d, 0x40);

        state.e = 0x80;
        state.set_flag_c(false);
        rr_e(&mut state);
        assert_eq!(state.e, 0x40);

        state.h = 0x80;
        state.set_flag_c(false);
        rr_h(&mut state);
        assert_eq!(state.h, 0x40);

        state.l = 0x80;
        state.set_flag_c(false);
        rr_l(&mut state);
        assert_eq!(state.l, 0x40);
    }

    #[test]
    fn test_rr_carry_propagation() {
        let mut state = State::new();
        state.a = 0xFF;
        state.set_flag_c(true);

        rr_a(&mut state);

        assert_eq!(state.a, 0xFF); // All bits set, carry in becomes bit 7
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 0 was 1
    }

    #[test]
    fn test_rra_always_resets_z() {
        let mut state = State::new();
        state.a = 0x00;
        state.set_flag_c(false);

        rra(&mut state);

        assert_eq!(state.a, 0x00);
        assert!(!state.flag_z()); // RRA always resets Z, even when result is 0
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_rra_normal() {
        let mut state = State::new();
        state.a = 0b1001_0101; // 0x95
        state.set_flag_c(true);

        rra(&mut state);

        assert_eq!(state.a, 0b1100_1010); // 0xCA (carry becomes bit 7)
        assert!(!state.flag_z()); // RRA always resets Z
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 0 was 1
    }

    #[test]
    fn test_rra_without_carry() {
        let mut state = State::new();
        state.a = 0b1001_0100; // 0x94
        state.set_flag_c(false);

        rra(&mut state);

        assert_eq!(state.a, 0b0100_1010); // 0x4A
        assert!(!state.flag_z()); // RRA always resets Z
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c()); // Bit 0 was 0
    }

    #[test]
    fn test_jr_nz_jumps_when_z_clear() {
        let mut state = State::new();
        state.pc = 0x1000;
        state.write(0x1000, 0x10); // Jump forward by 16 bytes
        state.set_flag_z(false); // Z flag clear

        jr_nz(&mut state);

        // Should jump: PC = 0x1000 + 1 + 0x10 = 0x1011
        assert_eq!(state.pc, 0x1011);
    }

    #[test]
    fn test_jr_nz_no_jump_when_z_set() {
        let mut state = State::new();
        state.pc = 0x1000;
        state.write(0x1000, 0x10); // Jump forward by 16 bytes
        state.set_flag_z(true); // Z flag set

        jr_nz(&mut state);

        // Should not jump: PC = 0x1000 + 1 = 0x1001
        assert_eq!(state.pc, 0x1001);
    }

    #[test]
    fn test_jr_nz_negative_offset() {
        let mut state = State::new();
        state.pc = 0x1000;
        state.write(0x1000, 0xFE); // Jump backward by 2 bytes (-2)
        state.set_flag_z(false); // Z flag clear

        jr_nz(&mut state);

        // Should jump: PC = 0x1000 + 1 + (-2) = 0x0FFF
        assert_eq!(state.pc, 0x0FFF);
    }

    #[test]
    fn test_jr_nz_zero_offset() {
        let mut state = State::new();
        state.pc = 0x1000;
        state.write(0x1000, 0x00); // No offset
        state.set_flag_z(false); // Z flag clear

        jr_nz(&mut state);

        // Should "jump" to same location: PC = 0x1000 + 1 + 0 = 0x1001
        assert_eq!(state.pc, 0x1001);
    }

    #[test]
    fn test_daa_after_add_no_adjust() {
        let mut state = State::new();
        state.a = 0x45; // BCD 45
        state.set_flag_n(false); // Addition
        state.set_flag_h(false);
        state.set_flag_c(false);

        daa(&mut state);

        assert_eq!(state.a, 0x45); // No adjustment needed
        assert!(!state.flag_z());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_daa_after_add_lower_nibble() {
        let mut state = State::new();
        state.a = 0x0F; // Lower nibble > 9
        state.set_flag_n(false); // Addition
        state.set_flag_h(false);
        state.set_flag_c(false);

        daa(&mut state);

        assert_eq!(state.a, 0x15); // 0x0F + 0x06 = 0x15
        assert!(!state.flag_z());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_daa_after_add_upper_nibble() {
        let mut state = State::new();
        state.a = 0x9A; // Upper nibble needs adjustment
        state.set_flag_n(false); // Addition
        state.set_flag_h(false);
        state.set_flag_c(false);

        daa(&mut state);

        assert_eq!(state.a, 0x00); // 0x9A + 0x60 = 0xFA, wraps to 0x00
        assert!(state.flag_z());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Carry set
    }

    #[test]
    fn test_daa_after_add_both_nibbles() {
        let mut state = State::new();
        state.a = 0x9F; // Both nibbles need adjustment
        state.set_flag_n(false); // Addition
        state.set_flag_h(false);
        state.set_flag_c(false);

        daa(&mut state);

        assert_eq!(state.a, 0x05); // 0x9F + 0x66 = 0x105, wraps to 0x05
        assert!(!state.flag_z());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Carry set
    }

    #[test]
    fn test_daa_after_add_with_half_carry() {
        let mut state = State::new();
        state.a = 0x12;
        state.set_flag_n(false); // Addition
        state.set_flag_h(true); // Half carry was set
        state.set_flag_c(false);

        daa(&mut state);

        assert_eq!(state.a, 0x18); // 0x12 + 0x06 = 0x18
        assert!(!state.flag_z());
        assert!(!state.flag_h()); // H flag reset by DAA
        assert!(!state.flag_c());
    }

    #[test]
    fn test_daa_after_add_with_carry() {
        let mut state = State::new();
        state.a = 0x12;
        state.set_flag_n(false); // Addition
        state.set_flag_h(false);
        state.set_flag_c(true); // Carry was set

        daa(&mut state);

        assert_eq!(state.a, 0x72); // 0x12 + 0x60 = 0x72
        assert!(!state.flag_z());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Carry remains set
    }

    #[test]
    fn test_daa_after_sub_no_adjust() {
        let mut state = State::new();
        state.a = 0x45;
        state.set_flag_n(true); // Subtraction
        state.set_flag_h(false);
        state.set_flag_c(false);

        daa(&mut state);

        assert_eq!(state.a, 0x45); // No adjustment needed
        assert!(!state.flag_z());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_daa_after_sub_with_half_carry() {
        let mut state = State::new();
        state.a = 0x50;
        state.set_flag_n(true); // Subtraction
        state.set_flag_h(true); // Half borrow
        state.set_flag_c(false);

        daa(&mut state);

        assert_eq!(state.a, 0x4A); // 0x50 - 0x06 = 0x4A
        assert!(!state.flag_z());
        assert!(!state.flag_h()); // H flag reset by DAA
        assert!(!state.flag_c());
    }

    #[test]
    fn test_daa_after_sub_with_carry() {
        let mut state = State::new();
        state.a = 0x70;
        state.set_flag_n(true); // Subtraction
        state.set_flag_h(false);
        state.set_flag_c(true); // Borrow

        daa(&mut state);

        assert_eq!(state.a, 0x10); // 0x70 - 0x60 = 0x10
        assert!(!state.flag_z());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Carry remains set
    }

    #[test]
    fn test_daa_zero_result() {
        let mut state = State::new();
        state.a = 0x00;
        state.set_flag_n(false);
        state.set_flag_h(false);
        state.set_flag_c(false);

        daa(&mut state);

        assert_eq!(state.a, 0x00);
        assert!(state.flag_z()); // Z flag set
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_has_pending_interrupt_none() {
        let mut state = State::new();
        state.write(IE, 0x00); // No interrupts enabled
        state.write(IF, 0x00); // No interrupts flagged

        assert!(!has_pending_interrupt(&state));
    }

    #[test]
    fn test_has_pending_interrupt_enabled_but_not_flagged() {
        let mut state = State::new();
        state.write(IE, 0x1F); // All interrupts enabled
        state.write(IF, 0x00); // No interrupts flagged

        assert!(!has_pending_interrupt(&state));
    }

    #[test]
    fn test_has_pending_interrupt_flagged_but_not_enabled() {
        let mut state = State::new();
        state.write(IE, 0x00); // No interrupts enabled
        state.write(IF, 0x1F); // All interrupts flagged

        assert!(!has_pending_interrupt(&state));
    }

    #[test]
    fn test_has_pending_interrupt_vblank() {
        let mut state = State::new();
        state.write(IE, 0x01); // V-Blank enabled (bit 0)
        state.write(IF, 0x01); // V-Blank flagged (bit 0)

        assert!(has_pending_interrupt(&state));
    }

    #[test]
    fn test_has_pending_interrupt_lcd_stat() {
        let mut state = State::new();
        state.write(IE, 0x02); // LCD STAT enabled (bit 1)
        state.write(IF, 0x02); // LCD STAT flagged (bit 1)

        assert!(has_pending_interrupt(&state));
    }

    #[test]
    fn test_has_pending_interrupt_timer() {
        let mut state = State::new();
        state.write(IE, 0x04); // Timer enabled (bit 2)
        state.write(IF, 0x04); // Timer flagged (bit 2)

        assert!(has_pending_interrupt(&state));
    }

    #[test]
    fn test_has_pending_interrupt_serial() {
        let mut state = State::new();
        state.write(IE, 0x08); // Serial enabled (bit 3)
        state.write(IF, 0x08); // Serial flagged (bit 3)

        assert!(has_pending_interrupt(&state));
    }

    #[test]
    fn test_has_pending_interrupt_joypad() {
        let mut state = State::new();
        state.write(IE, 0x10); // Joypad enabled (bit 4)
        state.write(IF, 0x10); // Joypad flagged (bit 4)

        assert!(has_pending_interrupt(&state));
    }

    #[test]
    fn test_has_pending_interrupt_multiple() {
        let mut state = State::new();
        state.write(IE, 0x1F); // All interrupts enabled
        state.write(IF, 0x03); // V-Blank and LCD STAT flagged

        assert!(has_pending_interrupt(&state));
    }

    #[test]
    fn test_has_pending_interrupt_partial_match() {
        let mut state = State::new();
        state.write(IE, 0x05); // V-Blank and Timer enabled
        state.write(IF, 0x07); // V-Blank, LCD STAT, and Timer flagged

        assert!(has_pending_interrupt(&state)); // Should match on V-Blank and Timer
    }

    #[test]
    fn test_has_pending_interrupt_ignores_upper_bits() {
        let mut state = State::new();
        state.write(IE, 0xFF); // All bits set
        state.write(IF, 0xE0); // Only upper bits set (not valid interrupts)

        assert!(!has_pending_interrupt(&state)); // Upper bits should be masked
    }

    #[test]
    fn test_service_interrupts_ime_disabled() {
        let mut state = State::new();
        state.ime = false;
        state.write(IE, 0x1F); // All interrupts enabled
        state.write(IF, 0x1F); // All interrupts flagged
        state.pc = 0x1000;

        let serviced = service_interrupts(&mut state);

        assert!(!serviced);
        assert_eq!(state.pc, 0x1000); // PC unchanged
        assert!(!state.ime); // IME still disabled
    }

    #[test]
    fn test_service_interrupts_no_pending() {
        let mut state = State::new();
        state.ime = true;
        state.write(IE, 0x00); // No interrupts enabled
        state.write(IF, 0x1F); // All interrupts flagged
        state.pc = 0x1000;

        let serviced = service_interrupts(&mut state);

        assert!(!serviced);
        assert_eq!(state.pc, 0x1000); // PC unchanged
        assert!(state.ime); // IME still enabled
    }

    #[test]
    fn test_service_interrupts_vblank() {
        let mut state = State::new();
        state.ime = true;
        state.write(IE, 0x01); // V-Blank enabled
        state.write(IF, 0x01); // V-Blank flagged
        state.pc = 0x1234;
        state.set_sp(0xFFFE);

        let serviced = service_interrupts(&mut state);

        assert!(serviced);
        assert_eq!(state.pc, 0x0040); // Jumped to V-Blank vector
        assert!(!state.ime); // IME disabled
        assert_eq!(state.read(IF), 0x00); // V-Blank flag cleared
        assert_eq!(state.sp(), 0xFFFC); // SP decremented by 2
        assert_eq!(state.read(0xFFFC), 0x34); // Low byte of PC pushed
        assert_eq!(state.read(0xFFFD), 0x12); // High byte of PC pushed
    }

    #[test]
    fn test_service_interrupts_lcd_stat() {
        let mut state = State::new();
        state.ime = true;
        state.write(IE, 0x02); // LCD STAT enabled
        state.write(IF, 0x02); // LCD STAT flagged
        state.pc = 0x5678;
        state.set_sp(0xC000);

        let serviced = service_interrupts(&mut state);

        assert!(serviced);
        assert_eq!(state.pc, 0x0048); // Jumped to LCD STAT vector
        assert!(!state.ime);
        assert_eq!(state.read(IF), 0x00);
    }

    #[test]
    fn test_service_interrupts_timer() {
        let mut state = State::new();
        state.ime = true;
        state.write(IE, 0x04); // Timer enabled
        state.write(IF, 0x04); // Timer flagged
        state.pc = 0xABCD;
        state.set_sp(0xD000);

        let serviced = service_interrupts(&mut state);

        assert!(serviced);
        assert_eq!(state.pc, 0x0050); // Jumped to Timer vector
        assert!(!state.ime);
        assert_eq!(state.read(IF), 0x00);
    }

    #[test]
    fn test_service_interrupts_serial() {
        let mut state = State::new();
        state.ime = true;
        state.write(IE, 0x08); // Serial enabled
        state.write(IF, 0x08); // Serial flagged
        state.pc = 0x2000;
        state.set_sp(0xE000);

        let serviced = service_interrupts(&mut state);

        assert!(serviced);
        assert_eq!(state.pc, 0x0058); // Jumped to Serial vector
        assert!(!state.ime);
        assert_eq!(state.read(IF), 0x00);
    }

    #[test]
    fn test_service_interrupts_joypad() {
        let mut state = State::new();
        state.ime = true;
        state.write(IE, 0x10); // Joypad enabled
        state.write(IF, 0x10); // Joypad flagged
        state.pc = 0x3000;
        state.set_sp(0xF000);

        let serviced = service_interrupts(&mut state);

        assert!(serviced);
        assert_eq!(state.pc, 0x0060); // Jumped to Joypad vector
        assert!(!state.ime);
        assert_eq!(state.read(IF), 0x00);
    }

    #[test]
    fn test_service_interrupts_priority_vblank_highest() {
        let mut state = State::new();
        state.ime = true;
        state.write(IE, 0x1F); // All interrupts enabled
        state.write(IF, 0x1F); // All interrupts flagged
        state.pc = 0x1000;
        state.set_sp(0xFFFE);

        let serviced = service_interrupts(&mut state);

        assert!(serviced);
        assert_eq!(state.pc, 0x0040); // V-Blank has highest priority
        assert_eq!(state.read(IF), 0x1E); // Only V-Blank flag cleared
    }

    #[test]
    fn test_service_interrupts_priority_lcd_stat_second() {
        let mut state = State::new();
        state.ime = true;
        state.write(IE, 0x1F); // All interrupts enabled
        state.write(IF, 0x1E); // All except V-Blank flagged
        state.pc = 0x1000;
        state.set_sp(0xFFFE);

        let serviced = service_interrupts(&mut state);

        assert!(serviced);
        assert_eq!(state.pc, 0x0048); // LCD STAT is next priority
        assert_eq!(state.read(IF), 0x1C); // Only LCD STAT flag cleared
    }

    #[test]
    fn test_service_interrupts_priority_joypad_lowest() {
        let mut state = State::new();
        state.ime = true;
        state.write(IE, 0x10); // Only Joypad enabled
        state.write(IF, 0x1F); // All interrupts flagged
        state.pc = 0x1000;
        state.set_sp(0xFFFE);

        let serviced = service_interrupts(&mut state);

        assert!(serviced);
        assert_eq!(state.pc, 0x0060); // Joypad handled because it's the only enabled one
        assert_eq!(state.read(IF), 0x0F); // Only Joypad flag cleared
    }

    #[test]
    fn test_service_interrupts_partial_flags_cleared() {
        let mut state = State::new();
        state.ime = true;
        state.write(IE, 0x05); // V-Blank and Timer enabled
        state.write(IF, 0x07); // V-Blank, LCD STAT, and Timer flagged
        state.pc = 0x1000;
        state.set_sp(0xFFFE);

        let serviced = service_interrupts(&mut state);

        assert!(serviced);
        assert_eq!(state.pc, 0x0040); // V-Blank serviced
        assert_eq!(state.read(IF), 0x06); // V-Blank cleared, LCD STAT and Timer remain
    }

    #[test]
    fn test_service_interrupts_stack_push_correct_order() {
        let mut state = State::new();
        state.ime = true;
        state.write(IE, 0x01);
        state.write(IF, 0x01);
        state.pc = 0xABCD;
        state.set_sp(0xC100);

        service_interrupts(&mut state);

        assert_eq!(state.sp(), 0xC0FE);
        assert_eq!(state.read(0xC0FE), 0xCD); // Low byte at lower address
        assert_eq!(state.read(0xC0FF), 0xAB); // High byte at higher address
    }
}
