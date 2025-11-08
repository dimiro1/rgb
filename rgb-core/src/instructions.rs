use crate::io::{IE, IF};
use crate::system::State;

// Opcode constants for special instructions
const OPCODE_DI: u8 = 0xF3; // Disable interrupts
const OPCODE_EI: u8 = 0xFB; // Enable interrupts

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

/// Bitwise AND an 8-bit value with register A and update flags accordingly
/// Z: Set if result is zero
/// N: Reset
/// H: Set (always)
/// C: Reset
fn and_a(value: u8, state: &mut State) {
    let result = state.a & value;

    state.set_flag_z(result == 0);
    state.set_flag_n(false);
    state.set_flag_h(true);
    state.set_flag_c(false);

    state.a = result;
}

/// Bitwise XOR an 8-bit value with register A and update flags accordingly
/// Z: Set if result is zero
/// N: Reset
/// H: Reset
/// C: Reset
fn xor_a(value: u8, state: &mut State) {
    let result = state.a ^ value;

    state.set_flag_z(result == 0);
    state.set_flag_n(false);
    state.set_flag_h(false);
    state.set_flag_c(false);

    state.a = result;
}

/// Bitwise OR an 8-bit value with register A and update flags accordingly
/// Z: Set if result is zero
/// N: Reset
/// H: Reset
/// C: Reset
fn or_a(value: u8, state: &mut State) {
    let result = state.a | value;

    state.set_flag_z(result == 0);
    state.set_flag_n(false);
    state.set_flag_h(false);
    state.set_flag_c(false);

    state.a = result;
}

/// Compare an 8-bit value with register A (like SUB but doesn't store result)
/// Z: Set if A == value (result is zero)
/// N: Set (subtraction operation)
/// H: Set if borrow from bit 4
/// C: Set if borrow (A < value)
fn cp_a(value: u8, state: &mut State) {
    let a = state.a;
    let result = a.wrapping_sub(value);

    state.set_flag_z(result == 0);
    state.set_flag_n(true);
    state.set_flag_h((a & 0xF) < (value & 0xF));
    state.set_flag_c(a < value);

    // Note: A register is NOT modified (that's the difference from SUB)
}

/// Read immediate byte from PC and advance PC
fn read_immediate_byte(state: &mut State) -> u8 {
    let value = state.read(state.pc);
    state.pc += 1;
    value
}

/// Read immediate 16-bit word from PC and advance PC (little-endian)
fn read_immediate_word(state: &mut State) -> u16 {
    let low = state.read(state.pc);
    state.pc += 1;
    let high = state.read(state.pc);
    state.pc += 1;
    ((high as u16) << 8) | (low as u16)
}

/// Pop word (16-bit) value from stack (little-endian)
fn pop_word(state: &mut State) -> u16 {
    // Pop low byte
    let low = state.read(state.sp);
    state.sp = state.sp.wrapping_add(1);

    // Pop high byte
    let high = state.read(state.sp);
    state.sp = state.sp.wrapping_add(1);

    // Return the 16-bit value (little-endian)
    ((high as u16) << 8) | (low as u16)
}

/// Return from subroutine - pop PC from stack
fn ret(state: &mut State) {
    state.pc = pop_word(state);
}

/// Return from subroutine if Z flag is clear (NZ)
fn ret_nz(state: &mut State) {
    if !state.flag_z() {
        ret(state);
    }
}

/// Return from subroutine if Z flag is set (Z)
fn ret_z(state: &mut State) {
    if state.flag_z() {
        ret(state);
    }
}

/// Return from subroutine if C flag is clear (NC)
fn ret_nc(state: &mut State) {
    if !state.flag_c() {
        ret(state);
    }
}

/// Return from subroutine if C flag is set (C)
fn ret_c(state: &mut State) {
    if state.flag_c() {
        ret(state);
    }
}

/// Return from interrupt - pop PC and enable interrupts
fn reti(state: &mut State) {
    ret(state);
    state.ime = true; // Enable interrupts
}

/// Pop 16-bit value from stack into BC register pair
fn pop_bc(state: &mut State) {
    let value = pop_word(state);
    state.c = value as u8; // Low byte
    state.b = (value >> 8) as u8; // High byte
}

/// Pop 16-bit value from stack into DE register pair
fn pop_de(state: &mut State) {
    let value = pop_word(state);
    state.e = value as u8; // Low byte
    state.d = (value >> 8) as u8; // High byte
}

/// Push DE register pair onto stack
fn push_de(state: &mut State) {
    let value = ((state.d as u16) << 8) | (state.e as u16);
    push_word(value, state);
}

/// Push BC register pair onto stack
fn push_bc(state: &mut State) {
    let value = ((state.b as u16) << 8) | (state.c as u16);
    push_word(value, state);
}

/// Jump to absolute 16-bit address
fn jp(state: &mut State) {
    let address = read_immediate_word(state);
    state.pc = address;
}

/// Jump to absolute address if Z flag is clear (NZ)
fn jp_nz(state: &mut State) {
    let address = read_immediate_word(state);
    if !state.flag_z() {
        state.pc = address;
    }
}

/// Jump to absolute address if Z flag is set (Z)
fn jp_z(state: &mut State) {
    let address = read_immediate_word(state);
    if state.flag_z() {
        state.pc = address;
    }
}

/// Jump to absolute address if C flag is clear (NC)
fn jp_nc(state: &mut State) {
    let address = read_immediate_word(state);
    if !state.flag_c() {
        state.pc = address;
    }
}

/// Jump to absolute address if C flag is set (C)
fn jp_c(state: &mut State) {
    let address = read_immediate_word(state);
    if state.flag_c() {
        state.pc = address;
    }
}

/// Push word (16-bit) value onto stack (little-endian)
fn push_word(value: u16, state: &mut State) {
    // Push high byte first
    state.sp = state.sp.wrapping_sub(1);
    state.write(state.sp, (value >> 8) as u8);

    // Push low byte
    state.sp = state.sp.wrapping_sub(1);
    state.write(state.sp, value as u8);
}

/// Call subroutine - push return address and jump to address
fn call(state: &mut State) {
    // Push return address (PC + 2, after reading the 2-byte address)
    push_word(state.pc + 2, state);
    // Jump to the target address
    jp(state);
}

/// Call subroutine if Z flag is clear (NZ)
fn call_nz(state: &mut State) {
    if !state.flag_z() {
        call(state);
    } else {
        // Skip the 2-byte address
        state.pc += 2;
    }
}

/// Call subroutine if Z flag is set (Z)
fn call_z(state: &mut State) {
    if state.flag_z() {
        call(state);
    } else {
        // Skip the 2-byte address
        state.pc += 2;
    }
}

/// Call subroutine if C flag is clear (NC)
fn call_nc(state: &mut State) {
    if !state.flag_c() {
        call(state);
    } else {
        // Skip the 2-byte address
        state.pc += 2;
    }
}

/// Call subroutine if C flag is set (C)
fn call_c(state: &mut State) {
    if state.flag_c() {
        call(state);
    } else {
        // Skip the 2-byte address
        state.pc += 2;
    }
}

/// RST 00h - Push PC and jump to address 0x0000
fn rst_00(state: &mut State) {
    push_word(state.pc, state);
    state.pc = 0x0000;
}

/// RST 08h - Push PC and jump to address 0x0008
fn rst_08(state: &mut State) {
    push_word(state.pc, state);
    state.pc = 0x0008;
}

/// RST 10h - Push PC and jump to address 0x0010
fn rst_10(state: &mut State) {
    push_word(state.pc, state);
    state.pc = 0x0010;
}

/// RST 18h - Push PC and jump to address 0x0018
fn rst_18(state: &mut State) {
    push_word(state.pc, state);
    state.pc = 0x0018;
}

/// Pop 16-bit value from stack into HL register pair
fn pop_hl(state: &mut State) {
    let value = pop_word(state);
    state.l = value as u8; // Low byte
    state.h = (value >> 8) as u8; // High byte
}

/// Push HL register pair onto stack
fn push_hl(state: &mut State) {
    let value = ((state.h as u16) << 8) | (state.l as u16);
    push_word(value, state);
}

/// LDH (n),A - Load A into high memory (0xFF00 + n)
fn ldh_n_a(state: &mut State) {
    let offset = read_immediate_byte(state);
    let address = 0xFF00 | (offset as u16);
    state.write(address, state.a);
}

/// LDH (C),A - Load A into high memory (0xFF00 + C)
fn ldh_c_a(state: &mut State) {
    let address = 0xFF00 | (state.c as u16);
    state.write(address, state.a);
}

/// Illegal/undefined opcode handler
/// Panics with error message showing the opcode and PC location
fn illegal_opcode(opcode: u8, state: &State) -> ! {
    panic!(
        "Illegal/undefined opcode 0x{:02X} at PC: 0x{:04X}",
        opcode,
        state.pc.wrapping_sub(1)
    );
}

/// RST 20h - Push PC and jump to address 0x0020
fn rst_20(state: &mut State) {
    push_word(state.pc, state);
    state.pc = 0x0020;
}

/// RST 28h - Push PC and jump to address 0x0028
fn rst_28(state: &mut State) {
    push_word(state.pc, state);
    state.pc = 0x0028;
}

/// RST 30h - Push PC and jump to address 0x0030
fn rst_30(state: &mut State) {
    push_word(state.pc, state);
    state.pc = 0x0030;
}

/// RST 38h - Push PC and jump to address 0x0038
fn rst_38(state: &mut State) {
    push_word(state.pc, state);
    state.pc = 0x0038;
}

/// ADD SP,n - Add signed immediate byte to SP
/// Z: Reset
/// N: Reset
/// H: Set if carry from bit 3
/// C: Set if carry from bit 7
fn add_sp_n(state: &mut State) {
    let offset = read_immediate_byte(state) as i8;
    let sp = state.sp;
    let result = sp.wrapping_add(offset as u16);

    // Flags are based on the lower byte addition
    state.set_flag_z(false);
    state.set_flag_n(false);
    state.set_flag_h((sp & 0xF) + ((offset as u16) & 0xF) > 0xF);
    state.set_flag_c((sp & 0xFF) + ((offset as u16) & 0xFF) > 0xFF);

    state.sp = result;
}

/// JP HL - Jump to address in HL
fn jp_hl(state: &mut State) {
    state.pc = state.hl();
}

/// LD (nn),A - Load A into memory at absolute address
fn ld_nn_a(state: &mut State) {
    let address = read_immediate_word(state);
    state.write(address, state.a);
}

/// LDH A,(n) - Load from high memory (0xFF00 + n) into A
fn ldh_a_n(state: &mut State) {
    let offset = read_immediate_byte(state);
    let address = 0xFF00 | (offset as u16);
    state.a = state.read(address);
}

/// LDH A,(C) - Load from high memory (0xFF00 + C) into A
fn ldh_a_c(state: &mut State) {
    let address = 0xFF00 | (state.c as u16);
    state.a = state.read(address);
}

/// LD HL,SP+n - Load SP plus signed immediate into HL
/// Z: Reset
/// N: Reset
/// H: Set if carry from bit 3
/// C: Set if carry from bit 7
fn ld_hl_sp_n(state: &mut State) {
    let offset = read_immediate_byte(state) as i8;
    let sp = state.sp;
    let result = sp.wrapping_add(offset as u16);

    // Flags are based on the lower byte addition
    state.set_flag_z(false);
    state.set_flag_n(false);
    state.set_flag_h((sp & 0xF) + ((offset as u16) & 0xF) > 0xF);
    state.set_flag_c((sp & 0xFF) + ((offset as u16) & 0xFF) > 0xFF);

    state.set_hl(result);
}

/// LD A,(nn) - Load from absolute address into A
fn ld_a_nn(state: &mut State) {
    let address = read_immediate_word(state);
    state.a = state.read(address);
}

/// Pop 16-bit value from stack into AF register pair
fn pop_af(state: &mut State) {
    let value = pop_word(state);
    state.f = value as u8; // Low byte (flags)
    state.a = (value >> 8) as u8; // High byte
}

/// Push AF register pair onto stack
fn push_af(state: &mut State) {
    let value = ((state.a as u16) << 8) | (state.f as u16);
    push_word(value, state);
}

/// HALT instruction - Enter low-power mode or trigger HALT bug
///
/// Normal behavior (IME=1 or no pending interrupts):
///   - CPU enters HALT mode and waits for an interrupt
///
/// HALT bug (IME=0 and pending interrupt):
///   - CPU exits HALT immediately but PC is NOT incremented
///   - This causes the next instruction byte to be read twice
fn halt(state: &mut State) {
    // Check for HALT bug: when IME=0 and there's a pending interrupt
    if !state.ime && has_pending_interrupt(state) {
        // HALT bug: CPU exits HALT immediately but PC is not incremented
        // This causes the next instruction byte to be read twice
        state.halt_bug = true;
    } else {
        // Normal HALT: CPU enters low-power mode
        state.halt = true;
    }
}

/// Handle delayed interrupt master enable (IME) changes
///
/// EI and DI instructions have a 1-instruction delay before taking effect.
/// This allows constructs like: EI; RETI to work correctly.
///
/// This function checks if the delayed IME change should be applied:
/// - If last instruction was NOT EI/DI, apply the delayed change
/// - If last instruction WAS EI/DI, keep the delay for one more instruction
///
/// Must be called BEFORE halt check so IME changes are processed even when halted
fn handle_delayed_ime(state: &mut State) {
    // Handle delayed interrupt disable (DI instruction)
    if state.di_delay {
        if state.last_opcode != OPCODE_DI {
            state.di_delay = false;
            state.ime = false;
        }
    }

    // Handle delayed interrupt enable (EI instruction)
    if state.ei_delay {
        if state.last_opcode != OPCODE_EI {
            state.ei_delay = false;
            state.ime = true;
        }
    }
}

/// Handle HALT mode processing at the beginning of instruction execution
///
/// This function handles both:
/// 1. HALT bug: Adjusts PC when halt_bug flag is set
/// 2. HALT exit: Checks if CPU should exit HALT mode due to interrupts
///
/// Returns true if instruction execution should continue, false if CPU is halted
///
/// HALT bug behavior (halt_bug = true):
///   - Decrements PC to cause next byte to be read twice
///   - Clears halt_bug flag
///   - Continues execution
///
/// HALT mode behavior (halt = true):
///   - If pending interrupt exists: exits HALT, continues execution
///   - If no pending interrupt: stays halted, returns false
///
/// Not halted:
///   - Returns true to continue normal execution
fn handle_halt(state: &mut State) -> bool {
    // Handle HALT bug: PC should not increment after HALT when bug is triggered
    // This causes the next instruction byte to be read twice
    if state.halt_bug {
        state.halt_bug = false;
        state.pc = state.pc.wrapping_sub(1); // Undo the PC increment from HALT instruction
        return true; // Continue execution (next byte will be read again)
    }

    // Handle HALT mode exit
    if state.halt {
        // Check if there are any pending interrupts
        if has_pending_interrupt(state) {
            state.halt = false;
            // If IME is enabled, the interrupt will be handled by service_interrupts()
            // If IME is disabled, we just continue execution (HALT exit behavior)
            return true; // Continue execution
        } else {
            // Still halted, don't execute instruction
            return false; // Skip execution
        }
    }

    // Not halted, continue normal execution
    true
}

/// Increment a byte value by 1 and update flags accordingly
fn inc_byte(value: u8, state: &mut State) -> u8 {
    let result = value.wrapping_add(1);
    state.set_flag_z(result == 0);
    state.set_flag_n(false);
    state.set_flag_h((value & 0xF) == 0xF);
    result
}

/// Increment the register A by 1 and update flags accordingly
fn inc_a(state: &mut State) {
    state.a = inc_byte(state.a, state);
}

// Increment the register B by 1 and update flags accordingly
fn inc_b(state: &mut State) {
    state.b = inc_byte(state.b, state);
}

// Increment the register C by 1 and update flags accordingly
fn inc_c(state: &mut State) {
    state.c = inc_byte(state.c, state);
}

/// Increment the register D by 1 and update flags accordingly
fn inc_d(state: &mut State) {
    state.d = inc_byte(state.d, state);
}

/// Increment the register E by 1 and update flags accordingly
fn inc_e(state: &mut State) {
    state.e = inc_byte(state.e, state);
}

/// Increment the register H by 1 and update flags accordingly
fn inc_h(state: &mut State) {
    state.h = inc_byte(state.h, state);
}

/// Increment the register L by 1 and update flags accordingly
fn inc_l(state: &mut State) {
    state.l = inc_byte(state.l, state);
}

/// Decrement a byte value by 1 and update flags accordingly
fn dec_byte(value: u8, state: &mut State) -> u8 {
    let result = value.wrapping_sub(1);
    state.set_flag_z(result == 0);
    state.set_flag_n(true);
    state.set_flag_h((value & 0xF) == 0);
    result
}

/// Decrement the register A by 1 and update flags accordingly
fn dec_a(state: &mut State) {
    state.a = dec_byte(state.a, state);
}

/// Decrement the register B by 1 and update flags accordingly
fn dec_b(state: &mut State) {
    state.b = dec_byte(state.b, state);
}

/// Decrement the register C by 1 and update flags accordingly
fn dec_c(state: &mut State) {
    state.c = dec_byte(state.c, state);
}

/// Decrement the register D by 1 and update flags accordingly
fn dec_d(state: &mut State) {
    state.d = dec_byte(state.d, state);
}

/// Decrement the register E by 1 and update flags accordingly
fn dec_e(state: &mut State) {
    state.e = dec_byte(state.e, state);
}

/// Decrement the register H by 1 and update flags accordingly
fn dec_h(state: &mut State) {
    state.h = dec_byte(state.h, state);
}

/// Decrement the register L by 1 and update flags accordingly
fn dec_l(state: &mut State) {
    state.l = dec_byte(state.l, state);
}

/// Rotate left circular (RLC) - rotates value left, bit 7 goes to carry and bit 0
fn rlc_byte(value: u8, state: &mut State) -> u8 {
    let bit7 = (value & 0x80) != 0;
    let result = (value << 1) | (if bit7 { 1 } else { 0 });

    state.set_flag_z(result == 0);
    state.set_flag_n(false);
    state.set_flag_h(false);
    state.set_flag_c(bit7);

    result
}

/// Rotate register A left circular
fn rlc_a(state: &mut State) {
    state.a = rlc_byte(state.a, state);
}

/// Rotate register B left circular
fn rlc_b(state: &mut State) {
    state.b = rlc_byte(state.b, state);
}

/// Rotate register C left circular
fn rlc_c(state: &mut State) {
    state.c = rlc_byte(state.c, state);
}

/// Rotate register D left circular
fn rlc_d(state: &mut State) {
    state.d = rlc_byte(state.d, state);
}

/// Rotate register E left circular
fn rlc_e(state: &mut State) {
    state.e = rlc_byte(state.e, state);
}

/// Rotate register H left circular
fn rlc_h(state: &mut State) {
    state.h = rlc_byte(state.h, state);
}

/// Rotate register L left circular
fn rlc_l(state: &mut State) {
    state.l = rlc_byte(state.l, state);
}

/// Rotate value at (HL) left circular
fn rlc_hl_indirect(state: &mut State) {
    let addr = state.hl();
    let value = state.read(addr);
    let result = rlc_byte(value, state);
    state.write(addr, result);
}

/// Rotate value at (HL) right circular
fn rrc_hl_indirect(state: &mut State) {
    let addr = state.hl();
    let value = state.read(addr);
    let result = rrc_byte(value, state);
    state.write(addr, result);
}

/// Rotate left through carry - value at (HL)
fn rl_hl_indirect(state: &mut State) {
    let addr = state.hl();
    let value = state.read(addr);
    let result = rl_byte(value, state);
    state.write(addr, result);
}

/// Rotate right through carry - value at (HL)
fn rr_hl_indirect(state: &mut State) {
    let addr = state.hl();
    let value = state.read(addr);
    let result = rr_byte(value, state);
    state.write(addr, result);
}

/// RLCA - Rotate A left circular (always resets Z flag)
fn rlca(state: &mut State) {
    state.a = rlc_byte(state.a, state);
    state.set_flag_z(false); // RLCA always resets Z flag
}

/// Rotate right circular (RRC) - rotates value right, bit 0 goes to carry and bit 7
fn rrc_byte(value: u8, state: &mut State) -> u8 {
    let bit0 = (value & 0x01) != 0;
    let result = (value >> 1) | (if bit0 { 0x80 } else { 0 });

    state.set_flag_z(result == 0);
    state.set_flag_n(false);
    state.set_flag_h(false);
    state.set_flag_c(bit0);

    result
}

/// Rotate register A right circular
fn rrc_a(state: &mut State) {
    state.a = rrc_byte(state.a, state);
}

/// Rotate register B right circular
fn rrc_b(state: &mut State) {
    state.b = rrc_byte(state.b, state);
}

/// Rotate register C right circular
fn rrc_c(state: &mut State) {
    state.c = rrc_byte(state.c, state);
}

/// Rotate register D right circular
fn rrc_d(state: &mut State) {
    state.d = rrc_byte(state.d, state);
}

/// Rotate register E right circular
fn rrc_e(state: &mut State) {
    state.e = rrc_byte(state.e, state);
}

/// Rotate register H right circular
fn rrc_h(state: &mut State) {
    state.h = rrc_byte(state.h, state);
}

/// Rotate register L right circular
fn rrc_l(state: &mut State) {
    state.l = rrc_byte(state.l, state);
}

/// RRCA - Rotate A right circular (always resets Z flag)
fn rrca(state: &mut State) {
    state.a = rrc_byte(state.a, state);
    state.set_flag_z(false); // RRCA always resets Z flag
}

/// Rotate left through carry (RL) - rotates value left through carry flag
/// Old carry goes to bit 0, bit 7 goes to carry
fn rl_byte(value: u8, state: &mut State) -> u8 {
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
fn rl_a(state: &mut State) {
    state.a = rl_byte(state.a, state);
}

/// Rotate register B left through carry
fn rl_b(state: &mut State) {
    state.b = rl_byte(state.b, state);
}

/// Rotate register C left through carry
fn rl_c(state: &mut State) {
    state.c = rl_byte(state.c, state);
}

/// Rotate register D left through carry
fn rl_d(state: &mut State) {
    state.d = rl_byte(state.d, state);
}

/// Rotate register E left through carry
fn rl_e(state: &mut State) {
    state.e = rl_byte(state.e, state);
}

/// Rotate register H left through carry
fn rl_h(state: &mut State) {
    state.h = rl_byte(state.h, state);
}

/// Rotate register L left through carry
fn rl_l(state: &mut State) {
    state.l = rl_byte(state.l, state);
}

/// RLA - Rotate A left through carry (always resets Z flag)
fn rla(state: &mut State) {
    state.a = rl_byte(state.a, state);
    state.set_flag_z(false); // RLA always resets Z flag
}

/// Rotate right through carry (RR) - rotates value right through carry flag
/// Old carry goes to bit 7, bit 0 goes to carry
fn rr_byte(value: u8, state: &mut State) -> u8 {
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
fn rr_a(state: &mut State) {
    state.a = rr_byte(state.a, state);
}

/// Rotate register B right through carry
fn rr_b(state: &mut State) {
    state.b = rr_byte(state.b, state);
}

/// Rotate register C right through carry
fn rr_c(state: &mut State) {
    state.c = rr_byte(state.c, state);
}

/// Rotate register D right through carry
fn rr_d(state: &mut State) {
    state.d = rr_byte(state.d, state);
}

/// Rotate register E right through carry
fn rr_e(state: &mut State) {
    state.e = rr_byte(state.e, state);
}

/// Rotate register H right through carry
fn rr_h(state: &mut State) {
    state.h = rr_byte(state.h, state);
}

/// Rotate register L right through carry
fn rr_l(state: &mut State) {
    state.l = rr_byte(state.l, state);
}

/// RRA - Rotate A right through carry (always resets Z flag)
fn rra(state: &mut State) {
    state.a = rr_byte(state.a, state);
    state.set_flag_z(false); // RRA always resets Z flag
}

/// SLA - Shift Left Arithmetic
/// Shifts value left, bit 7 goes to carry, bit 0 becomes 0
fn sla_byte(value: u8, state: &mut State) -> u8 {
    let bit7 = (value & 0x80) != 0;
    let result = value << 1;

    state.set_flag_z(result == 0);
    state.set_flag_n(false);
    state.set_flag_h(false);
    state.set_flag_c(bit7);

    result
}

/// Shift register A left arithmetic
fn sla_a(state: &mut State) {
    state.a = sla_byte(state.a, state);
}

/// Shift register B left arithmetic
fn sla_b(state: &mut State) {
    state.b = sla_byte(state.b, state);
}

/// Shift register C left arithmetic
fn sla_c(state: &mut State) {
    state.c = sla_byte(state.c, state);
}

/// Shift register D left arithmetic
fn sla_d(state: &mut State) {
    state.d = sla_byte(state.d, state);
}

/// Shift register E left arithmetic
fn sla_e(state: &mut State) {
    state.e = sla_byte(state.e, state);
}

/// Shift register H left arithmetic
fn sla_h(state: &mut State) {
    state.h = sla_byte(state.h, state);
}

/// Shift register L left arithmetic
fn sla_l(state: &mut State) {
    state.l = sla_byte(state.l, state);
}

/// Shift value at (HL) left arithmetic
fn sla_hl_indirect(state: &mut State) {
    let addr = state.hl();
    let value = state.read(addr);
    let result = sla_byte(value, state);
    state.write(addr, result);
}

/// SRA - Shift Right Arithmetic
/// Shifts value right, bit 0 goes to carry, bit 7 stays the same (preserves sign)
fn sra_byte(value: u8, state: &mut State) -> u8 {
    let bit0 = (value & 0x01) != 0;
    let bit7 = value & 0x80; // Preserve the sign bit
    let result = (value >> 1) | bit7;

    state.set_flag_z(result == 0);
    state.set_flag_n(false);
    state.set_flag_h(false);
    state.set_flag_c(bit0);

    result
}

/// Shift register A right arithmetic
fn sra_a(state: &mut State) {
    state.a = sra_byte(state.a, state);
}

/// Shift register B right arithmetic
fn sra_b(state: &mut State) {
    state.b = sra_byte(state.b, state);
}

/// Shift register C right arithmetic
fn sra_c(state: &mut State) {
    state.c = sra_byte(state.c, state);
}

/// Shift register D right arithmetic
fn sra_d(state: &mut State) {
    state.d = sra_byte(state.d, state);
}

/// Shift register E right arithmetic
fn sra_e(state: &mut State) {
    state.e = sra_byte(state.e, state);
}

/// Shift register H right arithmetic
fn sra_h(state: &mut State) {
    state.h = sra_byte(state.h, state);
}

/// Shift register L right arithmetic
fn sra_l(state: &mut State) {
    state.l = sra_byte(state.l, state);
}

/// Shift value at (HL) right arithmetic
fn sra_hl_indirect(state: &mut State) {
    let addr = state.hl();
    let value = state.read(addr);
    let result = sra_byte(value, state);
    state.write(addr, result);
}

/// SWAP - Swap upper and lower nibbles
/// Exchanges the upper 4 bits with the lower 4 bits
fn swap_byte(value: u8, state: &mut State) -> u8 {
    let result = ((value & 0x0F) << 4) | ((value & 0xF0) >> 4);

    state.set_flag_z(result == 0);
    state.set_flag_n(false);
    state.set_flag_h(false);
    state.set_flag_c(false);

    result
}

/// Swap register A nibbles
fn swap_a(state: &mut State) {
    state.a = swap_byte(state.a, state);
}

/// Swap register B nibbles
fn swap_b(state: &mut State) {
    state.b = swap_byte(state.b, state);
}

/// Swap register C nibbles
fn swap_c(state: &mut State) {
    state.c = swap_byte(state.c, state);
}

/// Swap register D nibbles
fn swap_d(state: &mut State) {
    state.d = swap_byte(state.d, state);
}

/// Swap register E nibbles
fn swap_e(state: &mut State) {
    state.e = swap_byte(state.e, state);
}

/// Swap register H nibbles
fn swap_h(state: &mut State) {
    state.h = swap_byte(state.h, state);
}

/// Swap register L nibbles
fn swap_l(state: &mut State) {
    state.l = swap_byte(state.l, state);
}

/// Swap value at (HL) nibbles
fn swap_hl_indirect(state: &mut State) {
    let addr = state.hl();
    let value = state.read(addr);
    let result = swap_byte(value, state);
    state.write(addr, result);
}

/// JR - Jump relative (unconditional)
/// Adds a signed 8-bit offset to PC
fn jr(state: &mut State) {
    let offset = read_immediate_byte(state) as i8;
    // Add the signed offset to PC
    state.pc = state.pc.wrapping_add(offset as u16);
}

/// JR NZ - Jump relative if not zero (Z flag is not set)
fn jr_nz(state: &mut State) {
    let offset = read_immediate_byte(state) as i8;

    if !state.flag_z() {
        state.pc = state.pc.wrapping_add(offset as u16);
    }
}

/// DAA - Decimal Adjust Accumulator
/// Adjusts the accumulator for BCD (Binary Coded Decimal) arithmetic
/// after addition or subtraction operations
/// see: https://blog.ollien.com/posts/gb-daa/
fn daa(state: &mut State) {
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
fn jr_z(state: &mut State) {
    let offset = read_immediate_byte(state) as i8;

    if state.flag_z() {
        state.pc = state.pc.wrapping_add(offset as u16);
    }
}

/// JR NC - Jump relative if not carry (C flag is not set)
fn jr_nc(state: &mut State) {
    let offset = read_immediate_byte(state) as i8;

    if !state.flag_c() {
        state.pc = state.pc.wrapping_add(offset as u16);
    }
}

/// JR C - Jump relative if carry (C flag is set)
fn jr_c(state: &mut State) {
    let offset = read_immediate_byte(state) as i8;

    if state.flag_c() {
        state.pc = state.pc.wrapping_add(offset as u16);
    }
}

/// CPL - Complement accumulator (flip all bits)
fn cpl(state: &mut State) {
    state.a = !state.a;
    state.set_flag_n(true);
    state.set_flag_h(true);
}

/// SCF - Set Carry Flag
fn scf(state: &mut State) {
    state.set_flag_c(true);
    state.set_flag_n(false);
    state.set_flag_h(false);
}

/// CCF - Complement Carry Flag
fn ccf(state: &mut State) {
    state.set_flag_c(!state.flag_c());
    state.set_flag_n(false);
    state.set_flag_h(false);
}

/// INC (HL) - Increment value at memory location pointed to by HL
fn inc_hl_indirect(state: &mut State) {
    let addr = state.hl();
    let value = state.read(addr);
    let result = inc_byte(value, state);
    state.write(addr, result);
}

/// DEC (HL) - Decrement value at memory location pointed to by HL
fn dec_hl_indirect(state: &mut State) {
    let addr = state.hl();
    let value = state.read(addr);
    let result = dec_byte(value, state);
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
fn add_hl_bc(state: &mut State) {
    let bc = state.bc();
    add_hl(bc, state);
}

/// ADD HL,DE - Add DE to HL
fn add_hl_de(state: &mut State) {
    let de = state.de();
    add_hl(de, state);
}

/// ADD HL,HL - Add HL to HL (double HL)
fn add_hl_hl(state: &mut State) {
    let hl = state.hl();
    add_hl(hl, state);
}

/// ADD HL,SP - Add SP to HL
fn add_hl_sp(state: &mut State) {
    let sp = state.sp();
    add_hl(sp, state);
}

/// Increment the BC register pair by 1
fn inc_bc(state: &mut State) {
    let value = state.bc().wrapping_add(1);
    state.set_bc(value);
}

/// Increment the DE register pair by 1
fn inc_de(state: &mut State) {
    let value = state.de().wrapping_add(1);
    state.set_de(value);
}

/// Increment the HL register pair by 1
fn inc_hl(state: &mut State) {
    let value = state.hl().wrapping_add(1);
    state.set_hl(value);
}

/// Increment the SP register by 1
fn inc_sp(state: &mut State) {
    let value = state.sp().wrapping_add(1);
    state.set_sp(value);
}

/// Decrement the BC register pair by 1
fn dec_bc(state: &mut State) {
    let value = state.bc().wrapping_sub(1);
    state.set_bc(value);
}

/// Decrement the DE register pair by 1
fn dec_de(state: &mut State) {
    let value = state.de().wrapping_sub(1);
    state.set_de(value);
}

/// Decrement the HL register pair by 1
fn dec_hl(state: &mut State) {
    let value = state.hl().wrapping_sub(1);
    state.set_hl(value);
}

/// Decrement the SP register by 1
fn dec_sp(state: &mut State) {
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
    handle_delayed_ime(state);

    // Handle HALT mode and HALT bug
    if !handle_halt(state) {
        // CPU is still halted, don't execute instruction
        return;
    }

    // TODO: This is not fully correct, in fact the read function must take into consideration the
    // current emomory bank and other detalis.
    let op = read_immediate_byte(state);
    state.last_opcode = op; // Store for delayed interrupt handling

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
            state.b = read_immediate_byte(state);
            state.cycles += 8;
        }
        0x07 => {
            /* RLCA */
            rlca(state);
            state.cycles += 4;
        }
        0x08 => {
            /* LD (nn),SP */
            let address = read_immediate_word(state);
            state.write_word(address, state.sp);
            state.cycles += 20;
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
            state.c = read_immediate_byte(state);
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
            state.d = read_immediate_byte(state);
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
            state.e = read_immediate_byte(state);
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
            state.h = read_immediate_byte(state);
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
            state.l = read_immediate_byte(state);
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
            let value = read_immediate_byte(state);
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
            state.a = read_immediate_byte(state);
            state.cycles += 8;
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
            halt(state);
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
        0xA0 => {
            /* AND B */
            and_a(state.b, state);
            state.cycles += 4;
        }
        0xA1 => {
            /* AND C */
            and_a(state.c, state);
            state.cycles += 4;
        }
        0xA2 => {
            /* AND D */
            and_a(state.d, state);
            state.cycles += 4;
        }
        0xA3 => {
            /* AND E */
            and_a(state.e, state);
            state.cycles += 4;
        }
        0xA4 => {
            /* AND H */
            and_a(state.h, state);
            state.cycles += 4;
        }
        0xA5 => {
            /* AND L */
            and_a(state.l, state);
            state.cycles += 4;
        }
        0xA6 => {
            /* AND (HL) */
            let value = state.read(state.hl());
            and_a(value, state);
            state.cycles += 8;
        }
        0xA7 => {
            /* AND A */
            and_a(state.a, state);
            state.cycles += 4;
        }
        0xA8 => {
            /* XOR B */
            xor_a(state.b, state);
            state.cycles += 4;
        }
        0xA9 => {
            /* XOR C */
            xor_a(state.c, state);
            state.cycles += 4;
        }
        0xAA => {
            /* XOR D */
            xor_a(state.d, state);
            state.cycles += 4;
        }
        0xAB => {
            /* XOR E */
            xor_a(state.e, state);
            state.cycles += 4;
        }
        0xAC => {
            /* XOR H */
            xor_a(state.h, state);
            state.cycles += 4;
        }
        0xAD => {
            /* XOR L */
            xor_a(state.l, state);
            state.cycles += 4;
        }
        0xAE => {
            /* XOR (HL) */
            let value = state.read(state.hl());
            xor_a(value, state);
            state.cycles += 8;
        }
        0xAF => {
            /* XOR A */
            xor_a(state.a, state);
            state.cycles += 4;
        }
        0xB0 => {
            /* OR B */
            or_a(state.b, state);
            state.cycles += 4;
        }
        0xB1 => {
            /* OR C */
            or_a(state.c, state);
            state.cycles += 4;
        }
        0xB2 => {
            /* OR D */
            or_a(state.d, state);
            state.cycles += 4;
        }
        0xB3 => {
            /* OR E */
            or_a(state.e, state);
            state.cycles += 4;
        }
        0xB4 => {
            /* OR H */
            or_a(state.h, state);
            state.cycles += 4;
        }
        0xB5 => {
            /* OR L */
            or_a(state.l, state);
            state.cycles += 4;
        }
        0xB6 => {
            /* OR (HL) */
            let value = state.read(state.hl());
            or_a(value, state);
            state.cycles += 8;
        }
        0xB7 => {
            /* OR A */
            or_a(state.a, state);
            state.cycles += 4;
        }
        0xB8 => {
            /* CP B */
            cp_a(state.b, state);
            state.cycles += 4;
        }
        0xB9 => {
            /* CP C */
            cp_a(state.c, state);
            state.cycles += 4;
        }
        0xBA => {
            /* CP D */
            cp_a(state.d, state);
            state.cycles += 4;
        }
        0xBB => {
            /* CP E */
            cp_a(state.e, state);
            state.cycles += 4;
        }
        0xBC => {
            /* CP H */
            cp_a(state.h, state);
            state.cycles += 4;
        }
        0xBD => {
            /* CP L */
            cp_a(state.l, state);
            state.cycles += 4;
        }
        0xBE => {
            /* CP (HL) */
            let value = state.read(state.hl());
            cp_a(value, state);
            state.cycles += 8;
        }
        0xBF => {
            /* CP A */
            cp_a(state.a, state);
            state.cycles += 4;
        }
        0xC0 => {
            /* RET NZ */
            ret_nz(state);
            // Conditional return: 8 cycles if not taken, 20 cycles if taken
            state.cycles += if !state.flag_z() { 20 } else { 8 };
        }
        0xC1 => {
            /* POP BC */
            pop_bc(state);
            state.cycles += 12;
        }
        0xC2 => {
            /* JP NZ */
            jp_nz(state);
            // Conditional jump: 12 cycles if not taken, 16 cycles if taken
            state.cycles += if !state.flag_z() { 16 } else { 12 };
        }
        0xC3 => {
            /* JP */
            jp(state);
            state.cycles += 16;
        }
        0xC4 => {
            /* CALL NZ */
            call_nz(state);
            // Conditional call: 12 cycles if not taken, 24 cycles if taken
            state.cycles += if !state.flag_z() { 24 } else { 12 };
        }
        0xC5 => {
            /* PUSH BC */
            push_bc(state);
            state.cycles += 16;
        }
        0xC6 => {
            /* ADD A,n */
            let value = read_immediate_byte(state);
            add_a(value, state);
            state.cycles += 8;
        }
        0xC7 => {
            /* RST 00H */
            rst_00(state);
            state.cycles += 16;
        }
        0xC8 => {
            /* RET Z */
            ret_z(state);
            // Conditional return: 8 cycles if not taken, 20 cycles if taken
            state.cycles += if state.flag_z() { 20 } else { 8 };
        }
        0xC9 => {
            /* RET */
            ret(state);
            state.cycles += 16;
        }
        0xCA => {
            /* JP Z */
            jp_z(state);
            // Conditional jump: 12 cycles if not taken, 16 cycles if taken
            state.cycles += if state.flag_z() { 16 } else { 12 };
        }
        0xCB => {
            /* CB prefix - Extended instruction set */
            let cb_op = read_immediate_byte(state);
            match cb_op {
                0x00 => {
                    /* RLC B */
                    rlc_b(state);
                    state.cycles += 8;
                }
                0x01 => {
                    /* RLC C */
                    rlc_c(state);
                    state.cycles += 8;
                }
                0x02 => {
                    /* RLC D */
                    rlc_d(state);
                    state.cycles += 8;
                }
                0x03 => {
                    /* RLC E */
                    rlc_e(state);
                    state.cycles += 8;
                }
                0x04 => {
                    /* RLC H */
                    rlc_h(state);
                    state.cycles += 8;
                }
                0x05 => {
                    /* RLC L */
                    rlc_l(state);
                    state.cycles += 8;
                }
                0x06 => {
                    /* RLC (HL) */
                    rlc_hl_indirect(state);
                    state.cycles += 16;
                }
                0x07 => {
                    /* RLC A */
                    rlc_a(state);
                    state.cycles += 8;
                }
                0x08 => {
                    /* RRC B */
                    rrc_b(state);
                    state.cycles += 8;
                }
                0x09 => {
                    /* RRC C */
                    rrc_c(state);
                    state.cycles += 8;
                }
                0x0A => {
                    /* RRC D */
                    rrc_d(state);
                    state.cycles += 8;
                }
                0x0B => {
                    /* RRC E */
                    rrc_e(state);
                    state.cycles += 8;
                }
                0x0C => {
                    /* RRC H */
                    rrc_h(state);
                    state.cycles += 8;
                }
                0x0D => {
                    /* RRC L */
                    rrc_l(state);
                    state.cycles += 8;
                }
                0x0E => {
                    /* RRC (HL) */
                    rrc_hl_indirect(state);
                    state.cycles += 16;
                }
                0x0F => {
                    /* RRC A */
                    rrc_a(state);
                    state.cycles += 8;
                }
                0x10 => {
                    /* RL B */
                    rl_b(state);
                    state.cycles += 8;
                }
                0x11 => {
                    /* RL C */
                    rl_c(state);
                    state.cycles += 8;
                }
                0x12 => {
                    /* RL D */
                    rl_d(state);
                    state.cycles += 8;
                }
                0x13 => {
                    /* RL E */
                    rl_e(state);
                    state.cycles += 8;
                }
                0x14 => {
                    /* RL H */
                    rl_h(state);
                    state.cycles += 8;
                }
                0x15 => {
                    /* RL L */
                    rl_l(state);
                    state.cycles += 8;
                }
                0x16 => {
                    /* RL (HL) */
                    rl_hl_indirect(state);
                    state.cycles += 16;
                }
                0x17 => {
                    /* RL A */
                    rl_a(state);
                    state.cycles += 8;
                }
                0x18 => {
                    /* RR B */
                    rr_b(state);
                    state.cycles += 8;
                }
                0x19 => {
                    /* RR C */
                    rr_c(state);
                    state.cycles += 8;
                }
                0x1A => {
                    /* RR D */
                    rr_d(state);
                    state.cycles += 8;
                }
                0x1B => {
                    /* RR E */
                    rr_e(state);
                    state.cycles += 8;
                }
                0x1C => {
                    /* RR H */
                    rr_h(state);
                    state.cycles += 8;
                }
                0x1D => {
                    /* RR L */
                    rr_l(state);
                    state.cycles += 8;
                }
                0x1E => {
                    /* RR (HL) */
                    rr_hl_indirect(state);
                    state.cycles += 16;
                }
                0x1F => {
                    /* RR A */
                    rr_a(state);
                    state.cycles += 8;
                }
                0x20 => {
                    /* SLA B */
                    sla_b(state);
                    state.cycles += 8;
                }
                0x21 => {
                    /* SLA C */
                    sla_c(state);
                    state.cycles += 8;
                }
                0x22 => {
                    /* SLA D */
                    sla_d(state);
                    state.cycles += 8;
                }
                0x23 => {
                    /* SLA E */
                    sla_e(state);
                    state.cycles += 8;
                }
                0x24 => {
                    /* SLA H */
                    sla_h(state);
                    state.cycles += 8;
                }
                0x25 => {
                    /* SLA L */
                    sla_l(state);
                    state.cycles += 8;
                }
                0x26 => {
                    /* SLA (HL) */
                    sla_hl_indirect(state);
                    state.cycles += 16;
                }
                0x27 => {
                    /* SLA A */
                    sla_a(state);
                    state.cycles += 8;
                }
                0x28 => {
                    /* SRA B */
                    sra_b(state);
                    state.cycles += 8;
                }
                0x29 => {
                    /* SRA C */
                    sra_c(state);
                    state.cycles += 8;
                }
                0x2A => {
                    /* SRA D */
                    sra_d(state);
                    state.cycles += 8;
                }
                0x2B => {
                    /* SRA E */
                    sra_e(state);
                    state.cycles += 8;
                }
                0x2C => {
                    /* SRA H */
                    sra_h(state);
                    state.cycles += 8;
                }
                0x2D => {
                    /* SRA L */
                    sra_l(state);
                    state.cycles += 8;
                }
                0x2E => {
                    /* SRA (HL) */
                    sra_hl_indirect(state);
                    state.cycles += 16;
                }
                0x2F => {
                    /* SRA A */
                    sra_a(state);
                    state.cycles += 8;
                }
                0x30 => {
                    /* SWAP B */
                    swap_b(state);
                    state.cycles += 8;
                }
                0x31 => {
                    /* SWAP C */
                    swap_c(state);
                    state.cycles += 8;
                }
                0x32 => {
                    /* SWAP D */
                    swap_d(state);
                    state.cycles += 8;
                }
                0x33 => {
                    /* SWAP E */
                    swap_e(state);
                    state.cycles += 8;
                }
                0x34 => {
                    /* SWAP H */
                    swap_h(state);
                    state.cycles += 8;
                }
                0x35 => {
                    /* SWAP L */
                    swap_l(state);
                    state.cycles += 8;
                }
                0x36 => {
                    /* SWAP (HL) */
                    swap_hl_indirect(state);
                    state.cycles += 16;
                }
                0x37 => {
                    /* SWAP A */
                    swap_a(state);
                    state.cycles += 8;
                }
                _ => {
                    panic!("Unimplemented CB opcode: 0x{:02X}", cb_op);
                }
            }
        }
        0xCC => {
            /* CALL Z */
            call_z(state);
            // Conditional call: 12 cycles if not taken, 24 cycles if taken
            state.cycles += if state.flag_z() { 24 } else { 12 };
        }
        0xCD => {
            /* CALL */
            call(state);
            state.cycles += 24;
        }
        0xCE => {
            /* ADC A,n */
            let value = read_immediate_byte(state);
            adc_a(value, state);
            state.cycles += 8;
        }
        0xCF => {
            /* RST 08h */
            rst_08(state);
            state.cycles += 16;
        }
        0xD0 => {
            /* RET NC */
            ret_nc(state);
            // Conditional return: 8 cycles if not taken, 20 cycles if taken
            state.cycles += if !state.flag_c() { 20 } else { 8 };
        }
        0xD1 => {
            /* POP DE */
            pop_de(state);
            state.cycles += 12;
        }
        0xD2 => {
            /* JP NC */
            jp_nc(state);
            // Conditional jump: 12 cycles if not taken, 16 cycles if taken
            state.cycles += if !state.flag_c() { 16 } else { 12 };
        }
        0xD3 => {
            /* Illegal opcode */
            illegal_opcode(0xD3, state);
        }
        0xD4 => {
            /* CALL NC */
            call_nc(state);
            // Conditional call: 12 cycles if not taken, 24 cycles if taken
            state.cycles += if !state.flag_c() { 24 } else { 12 };
        }
        0xD5 => {
            /* PUSH DE */
            push_de(state);
            state.cycles += 16;
        }
        0xD6 => {
            /* SUB n */
            let value = read_immediate_byte(state);
            sub_a(value, state);
            state.cycles += 8;
        }
        0xD7 => {
            /* RST 10h */
            rst_10(state);
            state.cycles += 16;
        }
        0xD8 => {
            /* RET C */
            ret_c(state);
            // Conditional return: 8 cycles if not taken, 20 cycles if taken
            state.cycles += if state.flag_c() { 20 } else { 8 };
        }
        0xD9 => {
            /* RETI */
            reti(state);
            state.cycles += 16;
        }
        0xDA => {
            /* JP C */
            jp_c(state);
            // Conditional jump: 12 cycles if not taken, 16 cycles if taken
            state.cycles += if state.flag_c() { 16 } else { 12 };
        }
        0xDB => {
            /* Illegal opcode */
            illegal_opcode(0xDB, state);
        }
        0xDC => {
            /* CALL C */
            call_c(state);
            // Conditional call: 12 cycles if not taken, 24 cycles if taken
            state.cycles += if state.flag_c() { 24 } else { 12 };
        }
        0xDD => {
            /* Illegal opcode */
            illegal_opcode(0xDD, state);
        }
        0xDE => {
            /* SBC A,n */
            let value = read_immediate_byte(state);
            sbc_a(value, state);
            state.cycles += 8;
        }
        0xDF => {
            /* RST 18h */
            rst_18(state);
            state.cycles += 16;
        }
        0xE0 => {
            /* LDH (n),A */
            ldh_n_a(state);
            state.cycles += 12;
        }
        0xE1 => {
            /* POP HL */
            pop_hl(state);
            state.cycles += 12;
        }
        0xE2 => {
            /* LDH (C),A */
            ldh_c_a(state);
            state.cycles += 8;
        }
        0xE3 => {
            /* Illegal opcode */
            illegal_opcode(0xE3, state);
        }
        0xE4 => {
            /* Illegal opcode */
            illegal_opcode(0xE4, state);
        }
        0xE5 => {
            /* PUSH HL */
            push_hl(state);
            state.cycles += 16;
        }
        0xE6 => {
            /* AND n */
            let value = read_immediate_byte(state);
            and_a(value, state);
            state.cycles += 8;
        }
        0xE7 => {
            /* RST 20h */
            rst_20(state);
            state.cycles += 16;
        }
        0xE8 => {
            /* ADD SP,n */
            add_sp_n(state);
            state.cycles += 16;
        }
        0xE9 => {
            /* JP HL */
            jp_hl(state);
            state.cycles += 4;
        }
        0xEA => {
            /* LD (nn),A */
            ld_nn_a(state);
            state.cycles += 16;
        }
        0xEB => {
            /* Illegal opcode */
            illegal_opcode(0xEB, state);
        }
        0xEC => {
            /* Illegal opcode */
            illegal_opcode(0xEC, state);
        }
        0xED => {
            /* Illegal opcode */
            illegal_opcode(0xED, state);
        }
        0xEE => {
            /* XOR n */
            let value = read_immediate_byte(state);
            xor_a(value, state);
            state.cycles += 8;
        }
        0xEF => {
            /* RST 28h */
            rst_28(state);
            state.cycles += 16;
        }
        0xF0 => {
            /* LDH A,(n) */
            ldh_a_n(state);
            state.cycles += 12;
        }
        0xF1 => {
            /* POP AF */
            pop_af(state);
            state.cycles += 12;
        }
        0xF2 => {
            /* LDH A,(C) */
            ldh_a_c(state);
            state.cycles += 8;
        }
        0xF3 => {
            /* DI */
            state.di_delay = true;
            state.cycles += 4;
        }
        0xF4 => {
            /* Illegal opcode */
            illegal_opcode(0xF4, state);
        }
        0xF5 => {
            /* PUSH AF */
            push_af(state);
            state.cycles += 16;
        }
        0xF6 => {
            /* OR n */
            let value = read_immediate_byte(state);
            or_a(value, state);
            state.cycles += 8;
        }
        0xF7 => {
            /* RST 30h */
            rst_30(state);
            state.cycles += 16;
        }
        0xF8 => {
            /* LD HL,SP+n */
            ld_hl_sp_n(state);
            state.cycles += 12;
        }
        0xF9 => {
            /* LD SP,HL */
            state.sp = state.hl();
            state.cycles += 8;
        }
        0xFA => {
            /* LD A,(nn) */
            ld_a_nn(state);
            state.cycles += 16;
        }
        0xFB => {
            /* EI */
            state.ei_delay = true;
            state.cycles += 4;
        }
        0xFC => {
            /* Illegal opcode */
            illegal_opcode(0xFC, state);
        }
        0xFD => {
            /* Illegal opcode */
            illegal_opcode(0xFD, state);
        }
        0xFE => {
            /* CP n */
            let value = read_immediate_byte(state);
            cp_a(value, state);
            state.cycles += 8;
        }
        0xFF => {
            /* RST 38h */
            rst_38(state);
            state.cycles += 16;
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

    // Tests for AND A,r
    #[test]
    fn test_and_a_normal() {
        let mut state = State::new();
        state.a = 0b11110000;

        and_a(0b10101010, &mut state);

        assert_eq!(state.a, 0b10100000);
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(state.flag_h()); // H always set for AND
        assert!(!state.flag_c());
    }

    #[test]
    fn test_and_a_zero_result() {
        let mut state = State::new();
        state.a = 0b11110000;

        and_a(0b00001111, &mut state);

        assert_eq!(state.a, 0x00);
        assert!(state.flag_z());
        assert!(!state.flag_n());
        assert!(state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_and_a_with_self() {
        let mut state = State::new();
        state.a = 0x5A;

        and_a(0x5A, &mut state);

        assert_eq!(state.a, 0x5A);
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_and_a_clears_carry() {
        let mut state = State::new();
        state.a = 0xFF;
        state.set_flag_c(true); // Set carry flag
        state.set_flag_n(true); // Set N flag

        and_a(0xFF, &mut state);

        assert_eq!(state.a, 0xFF);
        assert!(!state.flag_z());
        assert!(!state.flag_n()); // N cleared
        assert!(state.flag_h()); // H set
        assert!(!state.flag_c()); // C cleared
    }

    #[test]
    fn test_and_a_with_zero() {
        let mut state = State::new();
        state.a = 0xFF;

        and_a(0x00, &mut state);

        assert_eq!(state.a, 0x00);
        assert!(state.flag_z());
        assert!(!state.flag_n());
        assert!(state.flag_h());
        assert!(!state.flag_c());
    }

    // Tests for XOR A,r
    #[test]
    fn test_xor_a_normal() {
        let mut state = State::new();
        state.a = 0b11110000;

        xor_a(0b10101010, &mut state);

        assert_eq!(state.a, 0b01011010);
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_xor_a_with_self() {
        let mut state = State::new();
        state.a = 0x5A;

        xor_a(0x5A, &mut state);

        assert_eq!(state.a, 0x00); // XOR with self always gives 0
        assert!(state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_xor_a_with_zero() {
        let mut state = State::new();
        state.a = 0xFF;

        xor_a(0x00, &mut state);

        assert_eq!(state.a, 0xFF); // XOR with 0 doesn't change value
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_xor_a_clears_all_flags() {
        let mut state = State::new();
        state.a = 0xAA;
        state.set_flag_c(true); // Set carry flag
        state.set_flag_n(true); // Set N flag
        state.set_flag_h(true); // Set H flag

        xor_a(0x55, &mut state);

        assert_eq!(state.a, 0xFF);
        assert!(!state.flag_z());
        assert!(!state.flag_n()); // N cleared
        assert!(!state.flag_h()); // H cleared
        assert!(!state.flag_c()); // C cleared
    }

    #[test]
    fn test_xor_a_invert() {
        let mut state = State::new();
        state.a = 0b10101010;

        xor_a(0xFF, &mut state); // XOR with 0xFF inverts all bits

        assert_eq!(state.a, 0b01010101);
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    // Tests for OR A,r
    #[test]
    fn test_or_a_normal() {
        let mut state = State::new();
        state.a = 0b11110000;

        or_a(0b10101010, &mut state);

        assert_eq!(state.a, 0b11111010);
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_or_a_with_self() {
        let mut state = State::new();
        state.a = 0x5A;

        or_a(0x5A, &mut state);

        assert_eq!(state.a, 0x5A); // OR with self is identity
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_or_a_with_zero() {
        let mut state = State::new();
        state.a = 0xFF;

        or_a(0x00, &mut state);

        assert_eq!(state.a, 0xFF); // OR with 0 is identity
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_or_a_zero_result() {
        let mut state = State::new();
        state.a = 0x00;

        or_a(0x00, &mut state);

        assert_eq!(state.a, 0x00);
        assert!(state.flag_z()); // Zero result
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_or_a_clears_all_flags() {
        let mut state = State::new();
        state.a = 0x0F;
        state.set_flag_c(true); // Set carry flag
        state.set_flag_n(true); // Set N flag
        state.set_flag_h(true); // Set H flag

        or_a(0xF0, &mut state);

        assert_eq!(state.a, 0xFF);
        assert!(!state.flag_z());
        assert!(!state.flag_n()); // N cleared
        assert!(!state.flag_h()); // H cleared
        assert!(!state.flag_c()); // C cleared
    }

    // Tests for CP A,r (compare)
    #[test]
    fn test_cp_a_equal() {
        let mut state = State::new();
        state.a = 0x42;

        cp_a(0x42, &mut state);

        assert_eq!(state.a, 0x42); // A unchanged
        assert!(state.flag_z()); // Equal (A == value)
        assert!(state.flag_n()); // Subtraction
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_cp_a_greater_than() {
        let mut state = State::new();
        state.a = 0x50;

        cp_a(0x30, &mut state);

        assert_eq!(state.a, 0x50); // A unchanged
        assert!(!state.flag_z()); // Not equal
        assert!(state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c()); // No borrow (A > value)
    }

    #[test]
    fn test_cp_a_less_than() {
        let mut state = State::new();
        state.a = 0x30;

        cp_a(0x50, &mut state);

        assert_eq!(state.a, 0x30); // A unchanged
        assert!(!state.flag_z());
        assert!(state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Borrow (A < value)
    }

    #[test]
    fn test_cp_a_half_borrow() {
        let mut state = State::new();
        state.a = 0x3E;

        cp_a(0x0F, &mut state);

        assert_eq!(state.a, 0x3E); // A unchanged
        assert!(!state.flag_z());
        assert!(state.flag_n());
        assert!(state.flag_h()); // Half borrow: 0xE < 0xF
        assert!(!state.flag_c());
    }

    #[test]
    fn test_cp_a_with_zero() {
        let mut state = State::new();
        state.a = 0x00;

        cp_a(0x00, &mut state);

        assert_eq!(state.a, 0x00); // A unchanged
        assert!(state.flag_z()); // 0 == 0
        assert!(state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_cp_a_underflow() {
        let mut state = State::new();
        state.a = 0x00;

        cp_a(0x01, &mut state);

        assert_eq!(state.a, 0x00); // A unchanged (important!)
        assert!(!state.flag_z());
        assert!(state.flag_n());
        assert!(state.flag_h()); // Half borrow
        assert!(state.flag_c()); // Borrow
    }

    // Tests for RET instructions
    #[test]
    fn test_ret_nz_returns_when_z_clear() {
        let mut state = State::new();
        state.sp = 0xFFF0;
        state.pc = 0x1234;
        state.set_flag_z(false);

        // Setup stack with return address 0xABCD
        state.write(0xFFF0, 0xCD); // Low byte
        state.write(0xFFF1, 0xAB); // High byte

        ret_nz(&mut state);

        assert_eq!(state.pc, 0xABCD); // PC set to return address
        assert_eq!(state.sp, 0xFFF2); // SP incremented by 2
    }

    #[test]
    fn test_ret_nz_no_return_when_z_set() {
        let mut state = State::new();
        state.sp = 0xFFF0;
        state.pc = 0x1234;
        state.set_flag_z(true);

        // Setup stack with return address 0xABCD
        state.write(0xFFF0, 0xCD); // Low byte
        state.write(0xFFF1, 0xAB); // High byte

        ret_nz(&mut state);

        assert_eq!(state.pc, 0x1234); // PC unchanged
        assert_eq!(state.sp, 0xFFF0); // SP unchanged
    }

    #[test]
    fn test_ret_z_returns_when_z_set() {
        let mut state = State::new();
        state.sp = 0xFFF0;
        state.pc = 0x1234;
        state.set_flag_z(true);

        // Setup stack with return address 0x5678
        state.write(0xFFF0, 0x78); // Low byte
        state.write(0xFFF1, 0x56); // High byte

        ret_z(&mut state);

        assert_eq!(state.pc, 0x5678); // PC set to return address
        assert_eq!(state.sp, 0xFFF2); // SP incremented by 2
    }

    #[test]
    fn test_ret_z_no_return_when_z_clear() {
        let mut state = State::new();
        state.sp = 0xFFF0;
        state.pc = 0x1234;
        state.set_flag_z(false);

        // Setup stack with return address 0x5678
        state.write(0xFFF0, 0x78); // Low byte
        state.write(0xFFF1, 0x56); // High byte

        ret_z(&mut state);

        assert_eq!(state.pc, 0x1234); // PC unchanged
        assert_eq!(state.sp, 0xFFF0); // SP unchanged
    }

    #[test]
    fn test_ret_pops_correct_address() {
        let mut state = State::new();
        state.sp = 0x1000;

        // Test little-endian byte order
        state.write(0x1000, 0x34); // Low byte
        state.write(0x1001, 0x12); // High byte

        ret(&mut state);

        assert_eq!(state.pc, 0x1234);
        assert_eq!(state.sp, 0x1002);
    }

    #[test]
    fn test_ret_nc_returns_when_c_clear() {
        let mut state = State::new();
        state.sp = 0xFFF0;
        state.pc = 0x1234;
        state.set_flag_c(false);

        // Setup stack with return address 0xABCD
        state.write(0xFFF0, 0xCD); // Low byte
        state.write(0xFFF1, 0xAB); // High byte

        ret_nc(&mut state);

        assert_eq!(state.pc, 0xABCD); // PC set to return address
        assert_eq!(state.sp, 0xFFF2); // SP incremented by 2
    }

    #[test]
    fn test_ret_nc_no_return_when_c_set() {
        let mut state = State::new();
        state.sp = 0xFFF0;
        state.pc = 0x1234;
        state.set_flag_c(true);

        // Setup stack with return address 0xABCD
        state.write(0xFFF0, 0xCD); // Low byte
        state.write(0xFFF1, 0xAB); // High byte

        ret_nc(&mut state);

        assert_eq!(state.pc, 0x1234); // PC unchanged
        assert_eq!(state.sp, 0xFFF0); // SP unchanged
    }

    #[test]
    fn test_ret_c_returns_when_c_set() {
        let mut state = State::new();
        state.sp = 0xFFF0;
        state.pc = 0x1234;
        state.set_flag_c(true);

        // Setup stack with return address 0xABCD
        state.write(0xFFF0, 0xCD); // Low byte
        state.write(0xFFF1, 0xAB); // High byte

        ret_c(&mut state);

        assert_eq!(state.pc, 0xABCD); // PC set to return address
        assert_eq!(state.sp, 0xFFF2); // SP incremented by 2
    }

    #[test]
    fn test_ret_c_no_return_when_c_clear() {
        let mut state = State::new();
        state.sp = 0xFFF0;
        state.pc = 0x1234;
        state.set_flag_c(false);

        // Setup stack with return address 0xABCD
        state.write(0xFFF0, 0xCD); // Low byte
        state.write(0xFFF1, 0xAB); // High byte

        ret_c(&mut state);

        assert_eq!(state.pc, 0x1234); // PC unchanged
        assert_eq!(state.sp, 0xFFF0); // SP unchanged
    }

    #[test]
    fn test_reti_returns_and_enables_interrupts() {
        let mut state = State::new();
        state.sp = 0xFFF0;
        state.pc = 0x1234;
        state.ime = false; // Interrupts disabled

        // Setup stack with return address 0x5678
        state.write(0xFFF0, 0x78); // Low byte
        state.write(0xFFF1, 0x56); // High byte

        reti(&mut state);

        assert_eq!(state.pc, 0x5678); // PC set to return address
        assert_eq!(state.sp, 0xFFF2); // SP incremented by 2
        assert!(state.ime); // Interrupts enabled
    }

    // Tests for POP BC
    #[test]
    fn test_pop_bc_basic() {
        let mut state = State::new();
        state.sp = 0xFFF0;
        state.b = 0x00;
        state.c = 0x00;

        // Setup stack with value 0x1234
        state.write(0xFFF0, 0x34); // Low byte (C)
        state.write(0xFFF1, 0x12); // High byte (B)

        pop_bc(&mut state);

        assert_eq!(state.c, 0x34);
        assert_eq!(state.b, 0x12);
        assert_eq!(state.sp, 0xFFF2); // SP incremented by 2
    }

    #[test]
    fn test_pop_bc_little_endian() {
        let mut state = State::new();
        state.sp = 0x2000;

        // Test that bytes are popped in correct order (little-endian)
        state.write(0x2000, 0xCD); // Low byte goes to C
        state.write(0x2001, 0xAB); // High byte goes to B

        pop_bc(&mut state);

        assert_eq!(state.c, 0xCD);
        assert_eq!(state.b, 0xAB);
        assert_eq!(state.sp, 0x2002);
    }

    #[test]
    fn test_pop_bc_overwrites_previous_values() {
        let mut state = State::new();
        state.sp = 0x3000;
        state.b = 0xFF;
        state.c = 0xFF;

        state.write(0x3000, 0x11);
        state.write(0x3001, 0x22);

        pop_bc(&mut state);

        assert_eq!(state.c, 0x11);
        assert_eq!(state.b, 0x22);
    }

    // Tests for POP DE
    #[test]
    fn test_pop_de_basic() {
        let mut state = State::new();
        state.sp = 0xFFF0;
        state.d = 0x00;
        state.e = 0x00;

        // Setup stack with value 0x5678
        state.write(0xFFF0, 0x78); // Low byte (E)
        state.write(0xFFF1, 0x56); // High byte (D)

        pop_de(&mut state);

        assert_eq!(state.e, 0x78);
        assert_eq!(state.d, 0x56);
        assert_eq!(state.sp, 0xFFF2); // SP incremented by 2
    }

    // Tests for JP (absolute jump)
    #[test]
    fn test_jp_sets_pc_to_address() {
        let mut state = State::new();
        state.pc = 0x100;

        // Write jump address 0x8000 at PC (little-endian)
        state.write(0x100, 0x00); // Low byte
        state.write(0x101, 0x80); // High byte

        jp(&mut state);

        assert_eq!(state.pc, 0x8000);
    }

    #[test]
    fn test_jp_little_endian() {
        let mut state = State::new();
        state.pc = 0x200;

        // Write jump address 0x1234 (little-endian)
        state.write(0x200, 0x34); // Low byte
        state.write(0x201, 0x12); // High byte

        jp(&mut state);

        assert_eq!(state.pc, 0x1234);
    }

    #[test]
    fn test_jp_nz_jumps_when_z_clear() {
        let mut state = State::new();
        state.pc = 0x150;
        state.set_flag_z(false);

        state.write(0x150, 0xCD); // Low byte
        state.write(0x151, 0xAB); // High byte

        jp_nz(&mut state);

        assert_eq!(state.pc, 0xABCD);
    }

    #[test]
    fn test_jp_nz_no_jump_when_z_set() {
        let mut state = State::new();
        state.pc = 0x150;
        state.set_flag_z(true);

        state.write(0x150, 0xCD); // Low byte
        state.write(0x151, 0xAB); // High byte

        jp_nz(&mut state);

        // PC should be incremented by 2 (past the address bytes) but not jump
        assert_eq!(state.pc, 0x152);
    }

    #[test]
    fn test_jp_z_jumps_when_z_set() {
        let mut state = State::new();
        state.pc = 0x200;
        state.set_flag_z(true);

        state.write(0x200, 0x34); // Low byte
        state.write(0x201, 0x12); // High byte

        jp_z(&mut state);

        assert_eq!(state.pc, 0x1234);
    }

    #[test]
    fn test_jp_z_no_jump_when_z_clear() {
        let mut state = State::new();
        state.pc = 0x200;
        state.set_flag_z(false);

        state.write(0x200, 0x34); // Low byte
        state.write(0x201, 0x12); // High byte

        jp_z(&mut state);

        // PC should be incremented by 2 (past the address bytes) but not jump
        assert_eq!(state.pc, 0x202);
    }

    #[test]
    fn test_jp_nc_jumps_when_c_clear() {
        let mut state = State::new();
        state.pc = 0x200;
        state.set_flag_c(false);

        state.write(0x200, 0x00); // Low byte
        state.write(0x201, 0x30); // High byte

        jp_nc(&mut state);

        assert_eq!(state.pc, 0x3000);
    }

    #[test]
    fn test_jp_nc_no_jump_when_c_set() {
        let mut state = State::new();
        state.pc = 0x200;
        state.set_flag_c(true);

        state.write(0x200, 0x00); // Low byte
        state.write(0x201, 0x30); // High byte

        jp_nc(&mut state);

        // PC should be incremented by 2 (past the address bytes) but not jump
        assert_eq!(state.pc, 0x202);
    }

    #[test]
    fn test_jp_c_jumps_when_c_set() {
        let mut state = State::new();
        state.pc = 0x400;
        state.set_flag_c(true);

        state.write(0x400, 0x00); // Low byte
        state.write(0x401, 0x50); // High byte

        jp_c(&mut state);

        assert_eq!(state.pc, 0x5000);
    }

    #[test]
    fn test_jp_c_no_jump_when_c_clear() {
        let mut state = State::new();
        state.pc = 0x400;
        state.set_flag_c(false);

        state.write(0x400, 0x00); // Low byte
        state.write(0x401, 0x50); // High byte

        jp_c(&mut state);

        // PC should be incremented by 2 (past the address bytes) but not jump
        assert_eq!(state.pc, 0x402);
    }

    // Tests for CALL
    #[test]
    fn test_call_pushes_return_address_and_jumps() {
        let mut state = State::new();
        state.pc = 0x100;
        state.sp = 0xFFFE;

        // Write call target address 0x8000 at PC (little-endian)
        state.write(0x100, 0x00); // Low byte
        state.write(0x101, 0x80); // High byte

        call(&mut state);

        // PC should be at target address
        assert_eq!(state.pc, 0x8000);

        // Return address (0x102) should be pushed onto stack (little-endian)
        assert_eq!(state.sp, 0xFFFC);
        assert_eq!(state.read(0xFFFC), 0x02); // Low byte of return address
        assert_eq!(state.read(0xFFFD), 0x01); // High byte of return address
    }

    #[test]
    fn test_call_nz_calls_when_z_clear() {
        let mut state = State::new();
        state.pc = 0x200;
        state.sp = 0xFF00;
        state.set_flag_z(false);

        state.write(0x200, 0x34); // Low byte
        state.write(0x201, 0x12); // High byte

        call_nz(&mut state);

        // Should have called (jumped and pushed return address)
        assert_eq!(state.pc, 0x1234);
        assert_eq!(state.sp, 0xFEFE);
        assert_eq!(state.read(0xFEFE), 0x02); // Low byte of 0x202
        assert_eq!(state.read(0xFEFF), 0x02); // High byte of 0x202
    }

    #[test]
    fn test_call_nz_no_call_when_z_set() {
        let mut state = State::new();
        state.pc = 0x200;
        state.sp = 0xFF00;
        state.set_flag_z(true);

        state.write(0x200, 0x34); // Low byte
        state.write(0x201, 0x12); // High byte

        call_nz(&mut state);

        // Should not have called (PC advanced, SP unchanged)
        assert_eq!(state.pc, 0x202);
        assert_eq!(state.sp, 0xFF00);
    }

    #[test]
    fn test_call_z_calls_when_z_set() {
        let mut state = State::new();
        state.pc = 0x300;
        state.sp = 0xFF00;
        state.set_flag_z(true);

        state.write(0x300, 0x78); // Low byte
        state.write(0x301, 0x56); // High byte

        call_z(&mut state);

        // Should have called (jumped and pushed return address)
        assert_eq!(state.pc, 0x5678);
        assert_eq!(state.sp, 0xFEFE);
        assert_eq!(state.read(0xFEFE), 0x02); // Low byte of 0x302
        assert_eq!(state.read(0xFEFF), 0x03); // High byte of 0x302
    }

    #[test]
    fn test_call_z_no_call_when_z_clear() {
        let mut state = State::new();
        state.pc = 0x300;
        state.sp = 0xFF00;
        state.set_flag_z(false);

        state.write(0x300, 0x78); // Low byte
        state.write(0x301, 0x56); // High byte

        call_z(&mut state);

        // Should not have called (PC advanced, SP unchanged)
        assert_eq!(state.pc, 0x302);
        assert_eq!(state.sp, 0xFF00);
    }

    #[test]
    fn test_call_nc_calls_when_c_clear() {
        let mut state = State::new();
        state.pc = 0x300;
        state.sp = 0xFF00;
        state.set_flag_c(false);

        state.write(0x300, 0x56); // Low byte
        state.write(0x301, 0x34); // High byte

        call_nc(&mut state);

        // Should have called (jumped and pushed return address)
        assert_eq!(state.pc, 0x3456);
        assert_eq!(state.sp, 0xFEFE);
        assert_eq!(state.read(0xFEFE), 0x02); // Low byte of 0x302
        assert_eq!(state.read(0xFEFF), 0x03); // High byte of 0x302
    }

    #[test]
    fn test_call_nc_no_call_when_c_set() {
        let mut state = State::new();
        state.pc = 0x300;
        state.sp = 0xFF00;
        state.set_flag_c(true);

        state.write(0x300, 0x56); // Low byte
        state.write(0x301, 0x34); // High byte

        call_nc(&mut state);

        // Should not have called (PC advanced, SP unchanged)
        assert_eq!(state.pc, 0x302);
        assert_eq!(state.sp, 0xFF00);
    }

    #[test]
    fn test_call_c_calls_when_c_set() {
        let mut state = State::new();
        state.pc = 0x500;
        state.sp = 0xFF00;
        state.set_flag_c(true);

        state.write(0x500, 0x78); // Low byte
        state.write(0x501, 0x56); // High byte

        call_c(&mut state);

        // Should have called (jumped and pushed return address)
        assert_eq!(state.pc, 0x5678);
        assert_eq!(state.sp, 0xFEFE);
        assert_eq!(state.read(0xFEFE), 0x02); // Low byte of 0x502
        assert_eq!(state.read(0xFEFF), 0x05); // High byte of 0x502
    }

    #[test]
    fn test_call_c_no_call_when_c_clear() {
        let mut state = State::new();
        state.pc = 0x500;
        state.sp = 0xFF00;
        state.set_flag_c(false);

        state.write(0x500, 0x78); // Low byte
        state.write(0x501, 0x56); // High byte

        call_c(&mut state);

        // Should not have called (PC advanced, SP unchanged)
        assert_eq!(state.pc, 0x502);
        assert_eq!(state.sp, 0xFF00);
    }

    #[test]
    fn test_push_word_little_endian() {
        let mut state = State::new();
        state.sp = 0x2000;

        push_word(0xABCD, &mut state);

        // SP decremented by 2
        assert_eq!(state.sp, 0x1FFE);

        // Verify little-endian storage (low byte at lower address)
        assert_eq!(state.read(0x1FFE), 0xCD); // Low byte
        assert_eq!(state.read(0x1FFF), 0xAB); // High byte
    }

    // Tests for RST
    #[test]
    fn test_rst_00_pushes_pc_and_jumps() {
        let mut state = State::new();
        state.pc = 0xABCD;
        state.sp = 0xFFFE;

        rst_00(&mut state);

        // PC should be at RST vector 0x0000
        assert_eq!(state.pc, 0x0000);

        // Return address (0xABCD) should be pushed onto stack
        assert_eq!(state.sp, 0xFFFC);
        assert_eq!(state.read(0xFFFC), 0xCD); // Low byte
        assert_eq!(state.read(0xFFFD), 0xAB); // High byte
    }

    #[test]
    fn test_rst_08_pushes_pc_and_jumps() {
        let mut state = State::new();
        state.pc = 0x1234;
        state.sp = 0xFFFE;

        rst_08(&mut state);

        // PC should be at RST vector 0x0008
        assert_eq!(state.pc, 0x0008);

        // Return address (0x1234) should be pushed onto stack
        assert_eq!(state.sp, 0xFFFC);
        assert_eq!(state.read(0xFFFC), 0x34); // Low byte
        assert_eq!(state.read(0xFFFD), 0x12); // High byte
    }

    #[test]
    fn test_rst_10_pushes_pc_and_jumps() {
        let mut state = State::new();
        state.pc = 0x5678;
        state.sp = 0xFFFE;

        rst_10(&mut state);

        // PC should be at RST vector 0x0010
        assert_eq!(state.pc, 0x0010);

        // Return address (0x5678) should be pushed onto stack
        assert_eq!(state.sp, 0xFFFC);
        assert_eq!(state.read(0xFFFC), 0x78); // Low byte
        assert_eq!(state.read(0xFFFD), 0x56); // High byte
    }

    #[test]
    fn test_rst_18_pushes_pc_and_jumps() {
        let mut state = State::new();
        state.pc = 0x9ABC;
        state.sp = 0xFFFE;

        rst_18(&mut state);

        // PC should be at RST vector 0x0018
        assert_eq!(state.pc, 0x0018);

        // Return address (0x9ABC) should be pushed onto stack
        assert_eq!(state.sp, 0xFFFC);
        assert_eq!(state.read(0xFFFC), 0xBC); // Low byte
        assert_eq!(state.read(0xFFFD), 0x9A); // High byte
    }

    // Tests for PUSH DE
    #[test]
    fn test_push_de() {
        let mut state = State::new();
        state.sp = 0x3000;
        state.d = 0xAB;
        state.e = 0xCD;

        push_de(&mut state);

        // SP decremented by 2
        assert_eq!(state.sp, 0x2FFE);

        // Verify DE was pushed (little-endian)
        assert_eq!(state.read(0x2FFE), 0xCD); // Low byte (E)
        assert_eq!(state.read(0x2FFF), 0xAB); // High byte (D)
    }

    #[test]
    fn test_push_bc() {
        let mut state = State::new();
        state.sp = 0x3000;
        state.b = 0x12;
        state.c = 0x34;

        push_bc(&mut state);

        // SP decremented by 2
        assert_eq!(state.sp, 0x2FFE);

        // Verify BC was pushed (little-endian)
        assert_eq!(state.read(0x2FFE), 0x34); // Low byte (C)
        assert_eq!(state.read(0x2FFF), 0x12); // High byte (B)
    }

    // Tests for POP HL and PUSH HL
    #[test]
    fn test_pop_hl() {
        let mut state = State::new();
        state.sp = 0x2000;
        state.h = 0x00;
        state.l = 0x00;

        // Setup stack with value 0x1234
        state.write(0x2000, 0x34); // Low byte (L)
        state.write(0x2001, 0x12); // High byte (H)

        pop_hl(&mut state);

        assert_eq!(state.l, 0x34);
        assert_eq!(state.h, 0x12);
        assert_eq!(state.sp, 0x2002); // SP incremented by 2
    }

    #[test]
    fn test_push_hl() {
        let mut state = State::new();
        state.sp = 0x4000;
        state.h = 0x56;
        state.l = 0x78;

        push_hl(&mut state);

        // SP decremented by 2
        assert_eq!(state.sp, 0x3FFE);

        // Verify HL was pushed (little-endian)
        assert_eq!(state.read(0x3FFE), 0x78); // Low byte (L)
        assert_eq!(state.read(0x3FFF), 0x56); // High byte (H)
    }

    // Tests for LDH operations
    #[test]
    fn test_ldh_n_a() {
        let mut state = State::new();
        state.pc = 0x100;
        state.a = 0x42;

        // Write offset 0x80 at PC
        state.write(0x100, 0x80);

        ldh_n_a(&mut state);

        // Verify A was written to 0xFF80
        assert_eq!(state.read(0xFF80), 0x42);
        assert_eq!(state.pc, 0x101); // PC advanced by 1
    }

    #[test]
    fn test_ldh_c_a() {
        let mut state = State::new();
        state.a = 0x99;
        state.c = 0x44;

        ldh_c_a(&mut state);

        // Verify A was written to 0xFF44
        assert_eq!(state.read(0xFF44), 0x99);
    }

    #[test]
    fn test_inc_byte_normal() {
        let mut state = State::new();
        state.a = 0x42;

        inc_a(&mut state);

        assert_eq!(state.a, 0x43);
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
    }

    #[test]
    fn test_inc_byte_zero() {
        let mut state = State::new();
        state.b = 0xFF;

        inc_b(&mut state);

        assert_eq!(state.b, 0x00);
        assert!(state.flag_z()); // Result is zero
        assert!(!state.flag_n());
        assert!(state.flag_h()); // Half carry from 0xF to 0x0
    }

    #[test]
    fn test_inc_byte_half_carry() {
        let mut state = State::new();
        state.c = 0x0F;

        inc_c(&mut state);

        assert_eq!(state.c, 0x10);
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(state.flag_h()); // Half carry from bit 3 to bit 4
    }

    #[test]
    fn test_inc_byte_all_registers() {
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
    fn test_dec_byte_normal() {
        let mut state = State::new();
        state.a = 0x42;

        dec_a(&mut state);

        assert_eq!(state.a, 0x41);
        assert!(!state.flag_z());
        assert!(state.flag_n()); // N flag is set for subtraction
        assert!(!state.flag_h());
    }

    #[test]
    fn test_dec_byte_zero() {
        let mut state = State::new();
        state.b = 0x01;

        dec_b(&mut state);

        assert_eq!(state.b, 0x00);
        assert!(state.flag_z()); // Result is zero
        assert!(state.flag_n());
        assert!(!state.flag_h()); // No half-borrow
    }

    #[test]
    fn test_dec_byte_half_borrow() {
        let mut state = State::new();
        state.c = 0x10;

        dec_c(&mut state);

        assert_eq!(state.c, 0x0F);
        assert!(!state.flag_z());
        assert!(state.flag_n());
        assert!(state.flag_h()); // Half borrow from bit 4 to bit 3
    }

    #[test]
    fn test_dec_byte_underflow() {
        let mut state = State::new();
        state.d = 0x00;

        dec_d(&mut state);

        assert_eq!(state.d, 0xFF);
        assert!(!state.flag_z());
        assert!(state.flag_n());
        assert!(state.flag_h()); // Half borrow when decrementing from 0
    }

    #[test]
    fn test_dec_byte_all_registers() {
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
    fn test_rlc_hl_indirect() {
        let mut state = State::new();
        state.set_hl(0x1000);
        state.write(0x1000, 0b0100_1010); // 0x4A

        rlc_hl_indirect(&mut state);

        assert_eq!(state.read(0x1000), 0b1001_0100); // 0x94
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c()); // Bit 7 was 0
    }

    #[test]
    fn test_rrc_hl_indirect() {
        let mut state = State::new();
        state.set_hl(0x2000);
        state.write(0x2000, 0b1010_0101); // 0xA5

        rrc_hl_indirect(&mut state);

        assert_eq!(state.read(0x2000), 0b1101_0010); // 0xD2 - bit 0 rotated to bit 7
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 0 was 1
    }

    #[test]
    fn test_rrc_hl_indirect_zero_result() {
        let mut state = State::new();
        state.set_hl(0x3000);
        state.write(0x3000, 0x00);

        rrc_hl_indirect(&mut state);

        assert_eq!(state.read(0x3000), 0x00);
        assert!(state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_rl_hl_indirect_with_carry_clear() {
        let mut state = State::new();
        state.set_hl(0x4000);
        state.write(0x4000, 0b0100_1010); // 0x4A
        state.set_flag_c(false);

        rl_hl_indirect(&mut state);

        assert_eq!(state.read(0x4000), 0b1001_0100); // 0x94 - shifted left, carry in = 0
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c()); // Bit 7 was 0
    }

    #[test]
    fn test_rl_hl_indirect_with_carry_set() {
        let mut state = State::new();
        state.set_hl(0x5000);
        state.write(0x5000, 0b0100_1010); // 0x4A
        state.set_flag_c(true);

        rl_hl_indirect(&mut state);

        assert_eq!(state.read(0x5000), 0b1001_0101); // 0x95 - shifted left, carry in = 1
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c()); // Bit 7 was 0
    }

    #[test]
    fn test_rl_hl_indirect_sets_carry() {
        let mut state = State::new();
        state.set_hl(0x6000);
        state.write(0x6000, 0b1010_1010); // 0xAA
        state.set_flag_c(false);

        rl_hl_indirect(&mut state);

        assert_eq!(state.read(0x6000), 0b0101_0100); // 0x54 - bit 7 shifted out
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 7 was 1
    }

    #[test]
    fn test_rr_hl_indirect_with_carry_clear() {
        let mut state = State::new();
        state.set_hl(0x7000);
        state.write(0x7000, 0b1001_0100); // 0x94
        state.set_flag_c(false);

        rr_hl_indirect(&mut state);

        assert_eq!(state.read(0x7000), 0b0100_1010); // 0x4A - shifted right, carry in = 0
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c()); // Bit 0 was 0
    }

    #[test]
    fn test_rr_hl_indirect_with_carry_set() {
        let mut state = State::new();
        state.set_hl(0x8000);
        state.write(0x8000, 0b1001_0100); // 0x94
        state.set_flag_c(true);

        rr_hl_indirect(&mut state);

        assert_eq!(state.read(0x8000), 0b1100_1010); // 0xCA - shifted right, carry in = 1
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c()); // Bit 0 was 0
    }

    #[test]
    fn test_rr_hl_indirect_sets_carry() {
        let mut state = State::new();
        state.set_hl(0x9000);
        state.write(0x9000, 0b0101_0101); // 0x55
        state.set_flag_c(false);

        rr_hl_indirect(&mut state);

        assert_eq!(state.read(0x9000), 0b0010_1010); // 0x2A - bit 0 shifted out
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 0 was 1
    }

    #[test]
    fn test_rr_hl_indirect_zero_result() {
        let mut state = State::new();
        state.set_hl(0xA000);
        state.write(0xA000, 0x00);
        state.set_flag_c(false);

        rr_hl_indirect(&mut state);

        assert_eq!(state.read(0xA000), 0x00);
        assert!(state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_sla_hl_indirect() {
        let mut state = State::new();
        state.set_hl(0xB000);
        state.write(0xB000, 0b0100_1010); // 0x4A

        sla_hl_indirect(&mut state);

        assert_eq!(state.read(0xB000), 0b1001_0100); // 0x94 - shifted left, bit 0 = 0
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c()); // Bit 7 was 0
    }

    #[test]
    fn test_sla_hl_indirect_sets_carry() {
        let mut state = State::new();
        state.set_hl(0xC000);
        state.write(0xC000, 0b1010_1010); // 0xAA

        sla_hl_indirect(&mut state);

        assert_eq!(state.read(0xC000), 0b0101_0100); // 0x54 - bit 7 shifted out
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 7 was 1
    }

    #[test]
    fn test_sla_hl_indirect_zero_result() {
        let mut state = State::new();
        state.set_hl(0xD000);
        state.write(0xD000, 0x00);

        sla_hl_indirect(&mut state);

        assert_eq!(state.read(0xD000), 0x00);
        assert!(state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_sla_hl_indirect_overflow() {
        let mut state = State::new();
        state.set_hl(0xE000);
        state.write(0xE000, 0b1000_0000); // 0x80

        sla_hl_indirect(&mut state);

        assert_eq!(state.read(0xE000), 0x00); // Shifted out, result is 0
        assert!(state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 7 was 1
    }

    #[test]
    fn test_sra_hl_indirect_positive() {
        let mut state = State::new();
        state.set_hl(0xF000);
        state.write(0xF000, 0b0100_1010); // 0x4A - positive number (bit 7 = 0)

        sra_hl_indirect(&mut state);

        assert_eq!(state.read(0xF000), 0b0010_0101); // 0x25 - bit 7 stays 0
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c()); // Bit 0 was 0
    }

    #[test]
    fn test_sra_hl_indirect_negative() {
        let mut state = State::new();
        state.set_hl(0xF100);
        state.write(0xF100, 0b1010_1010); // 0xAA - negative number (bit 7 = 1)

        sra_hl_indirect(&mut state);

        assert_eq!(state.read(0xF100), 0b1101_0101); // 0xD5 - bit 7 stays 1 (preserves sign)
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c()); // Bit 0 was 0
    }

    #[test]
    fn test_sra_hl_indirect_sets_carry() {
        let mut state = State::new();
        state.set_hl(0xF200);
        state.write(0xF200, 0b0101_0101); // 0x55

        sra_hl_indirect(&mut state);

        assert_eq!(state.read(0xF200), 0b0010_1010); // 0x2A - bit 0 shifted out
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 0 was 1
    }

    #[test]
    fn test_sra_hl_indirect_zero_result() {
        let mut state = State::new();
        state.set_hl(0xF300);
        state.write(0xF300, 0x00);

        sra_hl_indirect(&mut state);

        assert_eq!(state.read(0xF300), 0x00);
        assert!(state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_sra_hl_indirect_preserves_sign_ff() {
        let mut state = State::new();
        state.set_hl(0xF400);
        state.write(0xF400, 0xFF); // All 1s

        sra_hl_indirect(&mut state);

        assert_eq!(state.read(0xF400), 0xFF); // Still all 1s (sign preserved)
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(state.flag_c()); // Bit 0 was 1
    }

    #[test]
    fn test_swap_hl_indirect() {
        let mut state = State::new();
        state.set_hl(0xF500);
        state.write(0xF500, 0x12); // Upper nibble = 1, lower nibble = 2

        swap_hl_indirect(&mut state);

        assert_eq!(state.read(0xF500), 0x21); // Swapped to 2 and 1
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c()); // SWAP always clears carry
    }

    #[test]
    fn test_swap_hl_indirect_zero_result() {
        let mut state = State::new();
        state.set_hl(0xF600);
        state.write(0xF600, 0x00);

        swap_hl_indirect(&mut state);

        assert_eq!(state.read(0xF600), 0x00);
        assert!(state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_swap_hl_indirect_symmetric() {
        let mut state = State::new();
        state.set_hl(0xF700);
        state.write(0xF700, 0xAB); // Upper = A, lower = B

        swap_hl_indirect(&mut state);

        assert_eq!(state.read(0xF700), 0xBA); // Upper = B, lower = A
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(!state.flag_h());
        assert!(!state.flag_c());
    }

    #[test]
    fn test_swap_hl_indirect_double_swap() {
        let mut state = State::new();
        state.set_hl(0xF800);
        state.write(0xF800, 0x34);

        swap_hl_indirect(&mut state);
        assert_eq!(state.read(0xF800), 0x43);

        swap_hl_indirect(&mut state);
        assert_eq!(state.read(0xF800), 0x34); // Back to original
    }

    #[test]
    fn test_swap_hl_indirect_clears_carry() {
        let mut state = State::new();
        state.set_hl(0xF900);
        state.write(0xF900, 0x56);
        state.set_flag_c(true); // Set carry before swap

        swap_hl_indirect(&mut state);

        assert_eq!(state.read(0xF900), 0x65);
        assert!(!state.flag_c()); // SWAP always clears carry
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

    // Tests for RST 20h and RST 28h
    #[test]
    fn test_rst_20_pushes_pc_and_jumps() {
        let mut state = State::new();
        state.pc = 0xABCD;
        state.sp = 0xFFFE;

        rst_20(&mut state);

        // PC should be at RST vector 0x0020
        assert_eq!(state.pc, 0x0020);

        // Return address (0xABCD) should be pushed onto stack
        assert_eq!(state.sp, 0xFFFC);
        assert_eq!(state.read(0xFFFC), 0xCD); // Low byte
        assert_eq!(state.read(0xFFFD), 0xAB); // High byte
    }

    #[test]
    fn test_rst_28_pushes_pc_and_jumps() {
        let mut state = State::new();
        state.pc = 0x1234;
        state.sp = 0xFFFE;

        rst_28(&mut state);

        // PC should be at RST vector 0x0028
        assert_eq!(state.pc, 0x0028);

        // Return address (0x1234) should be pushed onto stack
        assert_eq!(state.sp, 0xFFFC);
        assert_eq!(state.read(0xFFFC), 0x34); // Low byte
        assert_eq!(state.read(0xFFFD), 0x12); // High byte
    }

    // Tests for ADD SP,n
    #[test]
    fn test_add_sp_n_positive_offset() {
        let mut state = State::new();
        state.sp = 0x1000;
        state.pc = 0x100;
        state.write(0x100, 0x10); // Positive offset: +16

        add_sp_n(&mut state);

        assert_eq!(state.sp, 0x1010);
        assert!(!state.flag_z()); // Always reset
        assert!(!state.flag_n()); // Always reset
        assert_eq!(state.pc, 0x101); // PC advanced
    }

    #[test]
    fn test_add_sp_n_negative_offset() {
        let mut state = State::new();
        state.sp = 0x1000;
        state.pc = 0x100;
        state.write(0x100, 0xF0); // Negative offset: -16

        add_sp_n(&mut state);

        assert_eq!(state.sp, 0x0FF0);
        assert!(!state.flag_z()); // Always reset
        assert!(!state.flag_n()); // Always reset
    }

    #[test]
    fn test_add_sp_n_half_carry() {
        let mut state = State::new();
        state.sp = 0x000F;
        state.pc = 0x100;
        state.write(0x100, 0x01); // +1

        add_sp_n(&mut state);

        assert_eq!(state.sp, 0x0010);
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(state.flag_h()); // Half carry from bit 3
    }

    #[test]
    fn test_add_sp_n_carry() {
        let mut state = State::new();
        state.sp = 0x00FF;
        state.pc = 0x100;
        state.write(0x100, 0x01); // +1

        add_sp_n(&mut state);

        assert_eq!(state.sp, 0x0100);
        assert!(!state.flag_z());
        assert!(!state.flag_n());
        assert!(state.flag_c()); // Carry from bit 7
    }

    // Tests for JP HL
    #[test]
    fn test_jp_hl_jumps_to_hl() {
        let mut state = State::new();
        state.pc = 0x100;
        state.set_hl(0x8000);

        jp_hl(&mut state);

        assert_eq!(state.pc, 0x8000);
    }

    #[test]
    fn test_jp_hl_different_values() {
        let mut state = State::new();
        state.set_hl(0x1234);

        jp_hl(&mut state);

        assert_eq!(state.pc, 0x1234);
    }

    // Tests for LD (nn),A
    #[test]
    fn test_ld_nn_a() {
        let mut state = State::new();
        state.pc = 0x100;
        state.a = 0x42;

        // Write address 0x8000 at PC (little-endian)
        state.write(0x100, 0x00); // Low byte
        state.write(0x101, 0x80); // High byte

        ld_nn_a(&mut state);

        // Verify A was written to 0x8000
        assert_eq!(state.read(0x8000), 0x42);
        assert_eq!(state.pc, 0x102); // PC advanced by 2
    }

    #[test]
    fn test_ld_nn_a_different_address() {
        let mut state = State::new();
        state.pc = 0x200;
        state.a = 0xFF;

        state.write(0x200, 0x34); // Low byte
        state.write(0x201, 0x12); // High byte

        ld_nn_a(&mut state);

        assert_eq!(state.read(0x1234), 0xFF);
        assert_eq!(state.pc, 0x202);
    }

    // Tests for LDH A,(n) and LDH A,(C)
    #[test]
    fn test_ldh_a_n() {
        let mut state = State::new();
        state.pc = 0x100;
        state.a = 0x00;

        // Write offset 0x80 at PC
        state.write(0x100, 0x80);
        // Write test value at 0xFF80
        state.write(0xFF80, 0x42);

        ldh_a_n(&mut state);

        // Verify A was loaded from 0xFF80
        assert_eq!(state.a, 0x42);
        assert_eq!(state.pc, 0x101); // PC advanced by 1
    }

    #[test]
    fn test_ldh_a_c() {
        let mut state = State::new();
        state.a = 0x00;
        state.c = 0x44;

        // Write test value at 0xFF44
        state.write(0xFF44, 0x99);

        ldh_a_c(&mut state);

        // Verify A was loaded from 0xFF44
        assert_eq!(state.a, 0x99);
    }

    // Tests for POP AF
    #[test]
    fn test_pop_af_basic() {
        let mut state = State::new();
        state.sp = 0xFFF0;
        state.a = 0x00;
        state.f = 0x00;

        // Setup stack with value 0x12F0 (A=0x12, F=0xF0)
        state.write(0xFFF0, 0xF0); // Low byte (F)
        state.write(0xFFF1, 0x12); // High byte (A)

        pop_af(&mut state);

        assert_eq!(state.f, 0xF0);
        assert_eq!(state.a, 0x12);
        assert_eq!(state.sp, 0xFFF2); // SP incremented by 2
    }

    #[test]
    fn test_pop_af_restores_flags() {
        let mut state = State::new();
        state.sp = 0x2000;

        // Setup stack with flags: Z=1, N=1, H=1, C=1 (0xF0)
        state.write(0x2000, 0xF0); // F register
        state.write(0x2001, 0x42); // A register

        pop_af(&mut state);

        assert_eq!(state.a, 0x42);
        assert_eq!(state.f, 0xF0);
        assert!(state.flag_z());
        assert!(state.flag_n());
        assert!(state.flag_h());
        assert!(state.flag_c());
    }

    // Tests for HALT instruction
    #[test]
    fn test_halt_normal_with_ime_enabled() {
        let mut state = State::new();
        state.ime = true;
        state.halt = false;
        state.halt_bug = false;

        halt(&mut state);

        assert!(state.halt); // CPU enters HALT mode
        assert!(!state.halt_bug); // No HALT bug
    }

    #[test]
    fn test_halt_normal_with_ime_disabled_no_interrupt() {
        let mut state = State::new();
        state.ime = false;
        state.halt = false;
        state.halt_bug = false;
        // No pending interrupts (IE and IF are 0)

        halt(&mut state);

        assert!(state.halt); // CPU enters HALT mode
        assert!(!state.halt_bug); // No HALT bug
    }

    #[test]
    fn test_halt_bug_triggered() {
        use crate::io::{IE, IF};

        let mut state = State::new();
        state.ime = false; // Interrupts disabled
        state.halt = false;
        state.halt_bug = false;

        // Set up a pending interrupt (e.g., V-Blank)
        state.write(IE, 0x01); // Enable V-Blank interrupt
        state.write(IF, 0x01); // V-Blank interrupt pending

        halt(&mut state);

        assert!(!state.halt); // CPU does NOT enter HALT mode
        assert!(state.halt_bug); // HALT bug is triggered
    }

    #[test]
    fn test_halt_bug_with_multiple_pending_interrupts() {
        use crate::io::{IE, IF};

        let mut state = State::new();
        state.ime = false;
        state.halt = false;
        state.halt_bug = false;

        // Multiple pending interrupts
        state.write(IE, 0x1F); // Enable all interrupts
        state.write(IF, 0x0F); // Multiple interrupts pending

        halt(&mut state);

        assert!(!state.halt);
        assert!(state.halt_bug);
    }

    #[test]
    fn test_halt_no_bug_when_ime_enabled_with_pending_interrupt() {
        use crate::io::{IE, IF};

        let mut state = State::new();
        state.ime = true; // Interrupts enabled
        state.halt = false;
        state.halt_bug = false;

        // Pending interrupt
        state.write(IE, 0x01);
        state.write(IF, 0x01);

        halt(&mut state);

        assert!(state.halt); // Normal HALT behavior
        assert!(!state.halt_bug); // No bug when IME is enabled
    }

    #[test]
    fn test_halt_no_bug_when_interrupt_not_enabled() {
        use crate::io::{IE, IF};

        let mut state = State::new();
        state.ime = false;
        state.halt = false;
        state.halt_bug = false;

        // Interrupt pending but not enabled
        state.write(IE, 0x00); // No interrupts enabled
        state.write(IF, 0x01); // Interrupt pending

        halt(&mut state);

        assert!(state.halt); // Normal HALT
        assert!(!state.halt_bug); // No bug (interrupt not enabled in IE)
    }

    #[test]
    fn test_halt_no_bug_when_interrupt_enabled_but_not_pending() {
        use crate::io::{IE, IF};

        let mut state = State::new();
        state.ime = false;
        state.halt = false;
        state.halt_bug = false;

        // Interrupt enabled but not pending
        state.write(IE, 0x01); // Interrupt enabled
        state.write(IF, 0x00); // No interrupt pending

        halt(&mut state);

        assert!(state.halt); // Normal HALT
        assert!(!state.halt_bug); // No bug (no pending interrupt)
    }

    // Tests for handle_halt() function
    #[test]
    fn test_handle_halt_not_halted() {
        let mut state = State::new();
        state.halt = false;
        state.halt_bug = false;

        let should_continue = handle_halt(&mut state);

        assert!(should_continue); // Should continue execution
        assert!(!state.halt);
        assert!(!state.halt_bug);
    }

    #[test]
    fn test_handle_halt_bug_triggered() {
        let mut state = State::new();
        state.halt = false;
        state.halt_bug = true;
        state.pc = 0x100;

        let should_continue = handle_halt(&mut state);

        assert!(should_continue); // Should continue execution
        assert!(!state.halt_bug); // Flag cleared
        assert_eq!(state.pc, 0xFF); // PC decremented (100 - 1 = FF)
    }

    #[test]
    fn test_handle_halt_bug_with_wrapping() {
        let mut state = State::new();
        state.halt = false;
        state.halt_bug = true;
        state.pc = 0x0000;

        let should_continue = handle_halt(&mut state);

        assert!(should_continue);
        assert!(!state.halt_bug);
        assert_eq!(state.pc, 0xFFFF); // Wrapping: 0000 - 1 = FFFF
    }

    #[test]
    fn test_handle_halt_mode_with_interrupt() {
        use crate::io::{IE, IF};

        let mut state = State::new();
        state.halt = true;
        state.halt_bug = false;

        // Set up pending interrupt
        state.write(IE, 0x01);
        state.write(IF, 0x01);

        let should_continue = handle_halt(&mut state);

        assert!(should_continue); // Should exit HALT and continue
        assert!(!state.halt); // HALT mode exited
        assert!(!state.halt_bug);
    }

    #[test]
    fn test_handle_halt_mode_without_interrupt() {
        let mut state = State::new();
        state.halt = true;
        state.halt_bug = false;
        // No pending interrupts

        let should_continue = handle_halt(&mut state);

        assert!(!should_continue); // Should NOT continue (stay halted)
        assert!(state.halt); // Still in HALT mode
        assert!(!state.halt_bug);
    }

    #[test]
    fn test_handle_halt_bug_priority_over_halt_mode() {
        use crate::io::{IE, IF};

        let mut state = State::new();
        state.halt = true; // Both flags set
        state.halt_bug = true;
        state.pc = 0x200;

        // Set up pending interrupt
        state.write(IE, 0x01);
        state.write(IF, 0x01);

        let should_continue = handle_halt(&mut state);

        // HALT bug is handled first (checked before halt mode)
        assert!(should_continue);
        assert!(!state.halt_bug); // Bug flag cleared
        assert!(state.halt); // HALT flag NOT touched (bug takes priority)
        assert_eq!(state.pc, 0x1FF); // PC decremented
    }

    #[test]
    fn test_handle_halt_mode_exit_with_ime_enabled() {
        use crate::io::{IE, IF};

        let mut state = State::new();
        state.halt = true;
        state.ime = true; // IME enabled

        // Pending interrupt
        state.write(IE, 0x01);
        state.write(IF, 0x01);

        let should_continue = handle_halt(&mut state);

        assert!(should_continue);
        assert!(!state.halt); // Exited HALT
        // Interrupt will be serviced by service_interrupts() later
    }

    #[test]
    fn test_handle_halt_mode_exit_with_ime_disabled() {
        use crate::io::{IE, IF};

        let mut state = State::new();
        state.halt = true;
        state.ime = false; // IME disabled

        // Pending interrupt
        state.write(IE, 0x01);
        state.write(IF, 0x01);

        let should_continue = handle_halt(&mut state);

        assert!(should_continue);
        assert!(!state.halt); // Exited HALT
        // Execution continues without servicing interrupt (HALT exit behavior)
    }

    // Tests for handle_delayed_ime() function
    #[test]
    fn test_handle_delayed_ime_no_delay() {
        let mut state = State::new();
        state.ime = true;
        state.ei_delay = false;
        state.di_delay = false;
        state.last_opcode = 0x00; // NOP

        handle_delayed_ime(&mut state);

        assert!(state.ime); // IME unchanged
        assert!(!state.ei_delay);
        assert!(!state.di_delay);
    }

    #[test]
    fn test_handle_delayed_ime_di_applies_after_other_instruction() {
        let mut state = State::new();
        state.ime = true;
        state.di_delay = true;
        state.last_opcode = 0x00; // Last instruction was NOT DI

        handle_delayed_ime(&mut state);

        assert!(!state.ime); // IME disabled
        assert!(!state.di_delay); // Delay cleared
    }

    #[test]
    fn test_handle_delayed_ime_di_does_not_apply_after_di() {
        let mut state = State::new();
        state.ime = true;
        state.di_delay = true;
        state.last_opcode = OPCODE_DI; // Last instruction WAS DI

        handle_delayed_ime(&mut state);

        assert!(state.ime); // IME still enabled (delay continues)
        assert!(state.di_delay); // Delay flag still set
    }

    #[test]
    fn test_handle_delayed_ime_ei_applies_after_other_instruction() {
        let mut state = State::new();
        state.ime = false;
        state.ei_delay = true;
        state.last_opcode = 0x00; // Last instruction was NOT EI

        handle_delayed_ime(&mut state);

        assert!(state.ime); // IME enabled
        assert!(!state.ei_delay); // Delay cleared
    }

    #[test]
    fn test_handle_delayed_ime_ei_does_not_apply_after_ei() {
        let mut state = State::new();
        state.ime = false;
        state.ei_delay = true;
        state.last_opcode = OPCODE_EI; // Last instruction WAS EI

        handle_delayed_ime(&mut state);

        assert!(!state.ime); // IME still disabled (delay continues)
        assert!(state.ei_delay); // Delay flag still set
    }

    #[test]
    fn test_handle_delayed_ime_both_delays_set() {
        let mut state = State::new();
        state.ime = true;
        state.di_delay = true;
        state.ei_delay = true; // Both set (unusual but possible in testing)
        state.last_opcode = 0x00;

        handle_delayed_ime(&mut state);

        // DI is processed first (disables IME), then EI is processed (enables IME)
        assert!(state.ime); // EI applied last, so IME ends up enabled
        assert!(!state.di_delay); // DI delay cleared
        assert!(!state.ei_delay); // EI delay cleared
    }

    #[test]
    fn test_handle_delayed_ime_di_after_multiple_instructions() {
        let mut state = State::new();
        state.ime = true;
        state.di_delay = true;
        state.last_opcode = 0x3E; // LD A,n (random instruction)

        handle_delayed_ime(&mut state);

        assert!(!state.ime);
        assert!(!state.di_delay);
    }

    #[test]
    fn test_handle_delayed_ime_ei_after_reti() {
        let mut state = State::new();
        state.ime = false;
        state.ei_delay = true;
        state.last_opcode = 0xD9; // RETI (common pattern: EI; RETI)

        handle_delayed_ime(&mut state);

        assert!(state.ime); // Enabled after RETI
        assert!(!state.ei_delay);
    }

    #[test]
    fn test_handle_delayed_ime_preserves_ime_when_no_delay() {
        let mut state = State::new();
        state.ime = false;
        state.ei_delay = false;
        state.di_delay = false;
        state.last_opcode = 0x00;

        handle_delayed_ime(&mut state);

        assert!(!state.ime); // IME remains false
    }

    // Tests for PUSH AF
    #[test]
    fn test_push_af() {
        let mut state = State::new();
        state.sp = 0x3000;
        state.a = 0x42;
        state.f = 0xF0;

        push_af(&mut state);

        // SP decremented by 2
        assert_eq!(state.sp, 0x2FFE);

        // Verify AF was pushed (little-endian)
        assert_eq!(state.read(0x2FFE), 0xF0); // Low byte (F)
        assert_eq!(state.read(0x2FFF), 0x42); // High byte (A)
    }

    // Tests for RST 30h and RST 38h
    #[test]
    fn test_rst_30_pushes_pc_and_jumps() {
        let mut state = State::new();
        state.pc = 0x1234;
        state.sp = 0xFFFE;

        rst_30(&mut state);

        // PC should be at RST vector 0x0030
        assert_eq!(state.pc, 0x0030);

        // Return address (0x1234) should be pushed onto stack
        assert_eq!(state.sp, 0xFFFC);
        assert_eq!(state.read(0xFFFC), 0x34); // Low byte
        assert_eq!(state.read(0xFFFD), 0x12); // High byte
    }

    #[test]
    fn test_rst_38_pushes_pc_and_jumps() {
        let mut state = State::new();
        state.pc = 0xABCD;
        state.sp = 0xFFFE;

        rst_38(&mut state);

        // PC should be at RST vector 0x0038
        assert_eq!(state.pc, 0x0038);

        // Return address (0xABCD) should be pushed onto stack
        assert_eq!(state.sp, 0xFFFC);
        assert_eq!(state.read(0xFFFC), 0xCD); // Low byte
        assert_eq!(state.read(0xFFFD), 0xAB); // High byte
    }

    // Tests for LD HL,SP+n
    #[test]
    fn test_ld_hl_sp_n_positive_offset() {
        let mut state = State::new();
        state.sp = 0x1000;
        state.pc = 0x100;
        state.write(0x100, 0x10); // Positive offset: +16

        ld_hl_sp_n(&mut state);

        assert_eq!(state.hl(), 0x1010);
        assert!(!state.flag_z()); // Always reset
        assert!(!state.flag_n()); // Always reset
        assert_eq!(state.pc, 0x101); // PC advanced
    }

    #[test]
    fn test_ld_hl_sp_n_negative_offset() {
        let mut state = State::new();
        state.sp = 0x1000;
        state.pc = 0x100;
        state.write(0x100, 0xF0); // Negative offset: -16

        ld_hl_sp_n(&mut state);

        assert_eq!(state.hl(), 0x0FF0);
        assert!(!state.flag_z());
        assert!(!state.flag_n());
    }

    #[test]
    fn test_ld_hl_sp_n_half_carry() {
        let mut state = State::new();
        state.sp = 0x000F;
        state.pc = 0x100;
        state.write(0x100, 0x01); // +1

        ld_hl_sp_n(&mut state);

        assert_eq!(state.hl(), 0x0010);
        assert!(state.flag_h()); // Half carry from bit 3
    }

    #[test]
    fn test_ld_hl_sp_n_carry() {
        let mut state = State::new();
        state.sp = 0x00FF;
        state.pc = 0x100;
        state.write(0x100, 0x01); // +1

        ld_hl_sp_n(&mut state);

        assert_eq!(state.hl(), 0x0100);
        assert!(state.flag_c()); // Carry from bit 7
    }

    // Tests for LD A,(nn)
    #[test]
    fn test_ld_a_nn() {
        let mut state = State::new();
        state.pc = 0x100;
        state.a = 0x00;

        // Write address 0x8000 at PC (little-endian)
        state.write(0x100, 0x00); // Low byte
        state.write(0x101, 0x80); // High byte

        // Write test value at 0x8000
        state.write(0x8000, 0x42);

        ld_a_nn(&mut state);

        // Verify A was loaded from 0x8000
        assert_eq!(state.a, 0x42);
        assert_eq!(state.pc, 0x102); // PC advanced by 2
    }

    #[test]
    fn test_ld_a_nn_different_address() {
        let mut state = State::new();
        state.pc = 0x200;
        state.a = 0x00;

        state.write(0x200, 0x34); // Low byte
        state.write(0x201, 0x12); // High byte

        state.write(0x1234, 0xFF);

        ld_a_nn(&mut state);

        assert_eq!(state.a, 0xFF);
        assert_eq!(state.pc, 0x202);
    }
}
