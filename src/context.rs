/// Represents the GameBoy system state.
/// It holds the state of the whole system, including the CPU, memory and PPU state.
pub struct SystemState {
    pub a: u8,
    pub f: u8,
    pub h: u8,
    pub l: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub pc: u16,
    pub sp: u16,
}

impl SystemState {
    /// Initializes the CPU registers to their default power-on values.
    pub fn new() -> Self {
        let mut ctx = SystemState {
            a: 0,
            f: 0,
            h: 0,
            l: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            pc: 0,
            sp: 0,
        };
        ctx.set_af(0x01B0);
        ctx.set_bc(0x0013);
        ctx.set_de(0x00D8);
        ctx.set_hl(0x014D);
        ctx.set_sp(0xFFFE);
        ctx.set_pc(0x0100);
        ctx
    }

    /// Getter and setter for the AF register pair.
    pub fn af(&self) -> u16 {
        ((self.a as u16) << 8) | (self.f as u16)
    }

    /// Sets the AF register pair.
    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = value as u8;
    }

    /// Getter and setter for the BC register pair.
    pub fn bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }

    /// Sets the BC register pair.
    pub fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = value as u8;
    }

    /// Getter and setter for the DE register pair.
    pub fn de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }

    /// Sets the DE register pair.
    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = value as u8;
    }

    /// Getter and setter for the HL register pair.
    pub fn hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    /// Sets the HL register pair.
    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = value as u8;
    }

    /// Gets the stack pointer.
    pub fn sp(&self) -> u16 {
        self.sp
    }

    /// Sets the stack pointer.
    pub fn set_sp(&mut self, value: u16) {
        self.sp = value;
    }

    /// Gets the program counter.
    pub fn pc(&self) -> u16 {
        self.pc
    }

    /// Sets the program counter.
    pub fn set_pc(&mut self, value: u16) {
        self.pc = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_initializes_correct_values() {
        let state = SystemState::new();
        assert_eq!(state.af(), 0x01B0);
        assert_eq!(state.bc(), 0x0013);
        assert_eq!(state.de(), 0x00D8);
        assert_eq!(state.hl(), 0x014D);
        assert_eq!(state.sp(), 0xFFFE);
        assert_eq!(state.pc(), 0x0100);
    }

    #[test]
    fn test_register_pairs_getter_setter() {
        let mut state = SystemState::new();

        state.set_af(0xABCD);
        assert_eq!(state.af(), 0xABCD);

        state.set_bc(0x1234);
        assert_eq!(state.bc(), 0x1234);

        state.set_de(0x5678);
        assert_eq!(state.de(), 0x5678);

        state.set_hl(0x9ABC);
        assert_eq!(state.hl(), 0x9ABC);

        state.set_sp(0xFFFE);
        assert_eq!(state.sp(), 0xFFFE);

        state.set_pc(0x0100);
        assert_eq!(state.pc(), 0x0100);
    }
}
