use crate::cartridge::Cartridge;
use crate::memory::{FlatMemory, Memory};
use crate::mmu::Mmu;

/// Game Boy emulator
///
/// This is the new main structure that owns everything:
/// - CPU registers and state
/// - Memory (MMU for production, FlatMemory for tests)
///
/// This replaces the old `State` struct for new code.
pub struct GameBoy<M: Memory = Mmu> {
    // CPU Registers
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

    // CPU State
    pub ime: bool,       // Interrupt Master Enable flag
    pub halt: bool,      // CPU is halted
    pub halt_bug: bool,  // HALT bug triggered (PC not incremented after HALT)
    pub ei_delay: bool,  // EI takes effect after next instruction
    pub di_delay: bool,  // DI takes effect after next instruction
    pub cycles: u32,     // Total CPU cycles executed
    pub last_opcode: u8, // Last executed opcode (for delayed interrupt handling)

    // Memory (generic over Memory trait)
    pub mmu: M,
}

impl GameBoy<Mmu> {
    /// Create a new Game Boy with the given cartridge
    pub fn with_cartridge(cartridge: Cartridge) -> Self {
        let mut gb = GameBoy {
            // Initialize CPU registers to post-boot values
            a: 0x01,
            f: 0xB0,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            sp: 0xFFFE,
            pc: 0x0100,

            // CPU state
            ime: false,
            halt: false,
            halt_bug: false,
            ei_delay: false,
            di_delay: false,
            cycles: 0,
            last_opcode: 0,

            // MMU with cartridge
            mmu: Mmu::new(cartridge),
        };

        // Initialize I/O registers to post-boot values
        gb.init_io_registers();

        gb
    }

    /// Initialize I/O registers to their post-boot values
    fn init_io_registers(&mut self) {
        use crate::io::*;

        self.write(P1, 0xFF);
        self.write(DIV, 0xAF);
        self.write(TIMA, 0x00);
        self.write(TMA, 0x00);
        self.write(TAC, 0x00);
        self.write(NR_10, 0x80);
        self.write(NR_11, 0xBF);
        self.write(NR_12, 0xF3);
        self.write(NR_14, 0xBF);
        self.write(NR_21, 0x3F);
        self.write(NR_22, 0x00);
        self.write(NR_24, 0xBF);
        self.write(NR_30, 0x7F);
        self.write(NR_31, 0xFF);
        self.write(NR_32, 0x9F);
        self.write(NR_34, 0xBF);
        self.write(NR_41, 0xFF);
        self.write(NR_42, 0x00);
        self.write(NR_43, 0x00);
        self.write(NR_44, 0xBF);
        self.write(NR_50, 0x77);
        self.write(NR_51, 0xF3);
        self.write(NR_52, 0xF1);
        self.write(LCDC, 0x91);
        self.write(SCY, 0x00);
        self.write(SCX, 0x00);
        self.write(LYC, 0x00);
        self.write(BGP, 0xFC);
        self.write(OBP0, 0xFF);
        self.write(OBP1, 0xFF);
        self.write(WY, 0x00);
        self.write(WX, 0x00);
        self.write(IE, 0x00);
    }
}

// Generic implementation for all Memory types
impl<M: Memory> GameBoy<M> {
    /// Create a GameBoy with custom memory (for testing)
    pub fn with_memory(memory: M) -> Self {
        GameBoy {
            // Initialize CPU registers to post-boot values
            a: 0x01,
            f: 0xB0,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            sp: 0xFFFE,
            pc: 0x0100,

            // CPU state
            ime: false,
            halt: false,
            halt_bug: false,
            ei_delay: false,
            di_delay: false,
            cycles: 0,
            last_opcode: 0,

            mmu: memory,
        }
    }

    /// Read a byte from memory
    #[inline]
    pub fn read(&self, addr: u16) -> u8 {
        self.mmu.read(addr)
    }

    /// Write a byte to memory
    #[inline]
    pub fn write(&mut self, addr: u16, value: u8) {
        self.mmu.write(addr, value)
    }

    /// Read a 16-bit word from memory (little-endian)
    #[inline]
    pub fn read_word(&self, addr: u16) -> u16 {
        let low = self.read(addr) as u16;
        let high = self.read(addr + 1) as u16;
        (high << 8) | low
    }

    /// Write a 16-bit word to memory (little-endian)
    #[inline]
    pub fn write_word(&mut self, addr: u16, data: u16) {
        let high = (data >> 8) as u8;
        let low = (data & 0xFF) as u8;
        self.write(addr, low);
        self.write(addr + 1, high);
    }

    // Register pair getters
    #[inline]
    pub fn af(&self) -> u16 {
        ((self.a as u16) << 8) | (self.f as u16)
    }

    #[inline]
    pub fn bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }

    #[inline]
    pub fn de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }

    #[inline]
    pub fn hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    #[inline]
    pub fn sp(&self) -> u16 {
        self.sp
    }

    #[inline]
    pub fn pc(&self) -> u16 {
        self.pc
    }

    // Register pair setters
    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = (value & 0xF0) as u8; // Lower 4 bits always 0
    }

    pub fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = (value & 0xFF) as u8;
    }

    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = (value & 0xFF) as u8;
    }

    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = (value & 0xFF) as u8;
    }

    pub fn set_sp(&mut self, value: u16) {
        self.sp = value;
    }

    pub fn set_pc(&mut self, value: u16) {
        self.pc = value;
    }

    // Flag constants
    const FLAG_Z: u8 = 0b1000_0000; // Zero flag
    const FLAG_N: u8 = 0b0100_0000; // Subtract flag
    const FLAG_H: u8 = 0b0010_0000; // Half-carry flag
    const FLAG_C: u8 = 0b0001_0000; // Carry flag

    // Flag getters
    pub fn flag_z(&self) -> bool {
        self.f & Self::FLAG_Z != 0
    }

    pub fn set_flag_z(&mut self, value: bool) {
        if value {
            self.f |= Self::FLAG_Z;
        } else {
            self.f &= !Self::FLAG_Z;
        }
    }

    pub fn flag_n(&self) -> bool {
        self.f & Self::FLAG_N != 0
    }

    pub fn set_flag_n(&mut self, value: bool) {
        if value {
            self.f |= Self::FLAG_N;
        } else {
            self.f &= !Self::FLAG_N;
        }
    }

    pub fn flag_h(&self) -> bool {
        self.f & Self::FLAG_H != 0
    }

    pub fn set_flag_h(&mut self, value: bool) {
        if value {
            self.f |= Self::FLAG_H;
        } else {
            self.f &= !Self::FLAG_H;
        }
    }

    pub fn flag_c(&self) -> bool {
        self.f & Self::FLAG_C != 0
    }

    pub fn set_flag_c(&mut self, value: bool) {
        if value {
            self.f |= Self::FLAG_C;
        } else {
            self.f &= !Self::FLAG_C;
        }
    }
}

impl Default for GameBoy<Mmu> {
    fn default() -> Self {
        // Create a dummy ROM ONLY cartridge for production use
        let mut rom = vec![0; 64 * 1024]; // 64KB
        rom[0x0147] = 0x00; // ROM ONLY
        rom[0x0148] = 0x01; // 64 KiB
        rom[0x0149] = 0x00; // No RAM

        // Calculate header checksum
        let mut checksum: u8 = 0;
        for &byte in &rom[0x0134..=0x014C] {
            checksum = checksum.wrapping_sub(byte).wrapping_sub(1);
        }
        rom[0x014D] = checksum;

        let cartridge = Cartridge::from_bytes(rom).expect("Failed to create dummy cartridge");
        GameBoy::with_cartridge(cartridge)
    }
}

// Default implementation for tests uses FlatMemory
impl Default for GameBoy<FlatMemory> {
    fn default() -> Self {
        GameBoy::with_memory(FlatMemory::new())
    }
}

impl GameBoy<FlatMemory> {
    /// Create a new Game Boy instance with flat memory (for testing)
    pub fn new() -> Self {
        Self::default()
    }
}

/// Type alias for backward compatibility with tests
/// Uses FlatMemory for simple, writable memory without banking complexities
pub type State = GameBoy<FlatMemory>;

/// Update timers based on cycles executed
///
/// Game Boy timers:
/// - DIV (0xFF04): Divider register, increments at 16384 Hz (every 256 cycles)
/// - TIMA (0xFF05): Timer counter, increments at frequency set by TAC
/// - TMA (0xFF06): Timer modulo, loaded into TIMA when it overflows
/// - TAC (0xFF07): Timer control (bit 2 = enable, bits 0-1 = clock select)
pub fn update_timers(state: &mut State, cycles: u32) {
    use crate::io::{DIV, IF, TAC, TIMA, TMA};

    // Update DIV register (increments every 256 cycles = 16384 Hz)
    // DIV is incremented by internal counter, we track using a simplified approach
    let div_increments = cycles / 256;
    if div_increments > 0 {
        let current_div = state.read(DIV);
        state.write(DIV, current_div.wrapping_add(div_increments as u8));
    }

    // Check if timer is enabled (bit 2 of TAC)
    let tac = state.read(TAC);
    let timer_enabled = (tac & 0x04) != 0;

    if timer_enabled {
        // Clock select (bits 0-1 of TAC):
        // 00: 4096 Hz   (1024 cycles per increment)
        // 01: 262144 Hz (16 cycles per increment)
        // 10: 65536 Hz  (64 cycles per increment)
        // 11: 16384 Hz  (256 cycles per increment)
        let clock_select = tac & 0x03;
        let cycles_per_increment = match clock_select {
            0 => 1024,
            1 => 16,
            2 => 64,
            3 => 256,
            _ => unreachable!(),
        };

        let tima_increments = cycles / cycles_per_increment;
        if tima_increments > 0 {
            let mut tima = state.read(TIMA);

            for _ in 0..tima_increments {
                tima = tima.wrapping_add(1);

                // Check for overflow (wraparound from 0xFF to 0x00)
                if tima == 0 {
                    // Reload from TMA
                    tima = state.read(TMA);

                    // Request timer interrupt (bit 2 of IF)
                    let if_flags = state.read(IF);
                    state.write(IF, if_flags | 0x04);
                }
            }

            state.write(TIMA, tima);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_initializes_correct_values() {
        let state = State::new();
        assert_eq!(state.af(), 0x01B0);
        assert_eq!(state.bc(), 0x0013);
        assert_eq!(state.de(), 0x00D8);
        assert_eq!(state.hl(), 0x014D);
        assert_eq!(state.sp(), 0xFFFE);
        assert_eq!(state.pc(), 0x0100);
    }

    #[test]
    fn test_register_pairs_getter_setter() {
        let mut state = State::new();

        state.set_af(0xABCD);
        assert_eq!(state.af(), 0xABC0); // Lower 4 bits of F are always 0

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

    #[test]
    fn test_memory_read_write() {
        let mut state = State::new();

        // Test writing and reading at various addresses
        state.write(0x0000, 0xAB);
        assert_eq!(state.read(0x0000), 0xAB);

        state.write(0x1234, 0xCD);
        assert_eq!(state.read(0x1234), 0xCD);

        state.write(0xFF00, 0x12);
        assert_eq!(state.read(0xFF00), 0x12);

        state.write(0xFFFF, 0x34);
        assert_eq!(state.read(0xFFFF), 0x34);
    }

    #[test]
    fn test_read_word() {
        let mut state = State::new();

        // Test reading a word (little-endian: low byte first, high byte second)
        state.write(0x1000, 0x34); // Low byte
        state.write(0x1001, 0x12); // High byte
        assert_eq!(state.read_word(0x1000), 0x1234);

        // Test at different address
        state.write(0x2000, 0xCD);
        state.write(0x2001, 0xAB);
        assert_eq!(state.read_word(0x2000), 0xABCD);

        // Test at boundary
        state.write(0xFFFE, 0xFF);
        state.write(0xFFFF, 0xFF);
        assert_eq!(state.read_word(0xFFFE), 0xFFFF);
    }

    #[test]
    fn test_write_word() {
        let mut state = State::new();

        // Test writing a word (little-endian: low byte first, high byte second)
        state.write_word(0x1000, 0x1234);
        assert_eq!(state.read(0x1000), 0x34); // Low byte
        assert_eq!(state.read(0x1001), 0x12); // High byte

        // Test at different address
        state.write_word(0x2000, 0xABCD);
        assert_eq!(state.read(0x2000), 0xCD); // Low byte
        assert_eq!(state.read(0x2001), 0xAB); // High byte

        // Test at boundary
        state.write_word(0xFFFE, 0xFFFF);
        assert_eq!(state.read(0xFFFE), 0xFF);
        assert_eq!(state.read(0xFFFF), 0xFF);
    }

    #[test]
    fn test_read_write_word_roundtrip() {
        let mut state = State::new();

        // Test that write_word and read_word are inverses
        state.write_word(0x3000, 0x5678);
        assert_eq!(state.read_word(0x3000), 0x5678);

        state.write_word(0x4000, 0x0000);
        assert_eq!(state.read_word(0x4000), 0x0000);

        state.write_word(0x5000, 0xFFFF);
        assert_eq!(state.read_word(0x5000), 0xFFFF);
    }

    #[test]
    fn test_update_timers_div() {
        use crate::io::DIV;
        let mut state = State::new();

        state.write(DIV, 0x00);

        // Run 256 cycles - should increment DIV by 1
        update_timers(&mut state, 256);
        assert_eq!(state.read(DIV), 0x01);

        // Run 512 more cycles - should increment DIV by 2
        update_timers(&mut state, 512);
        assert_eq!(state.read(DIV), 0x03);
    }

    #[test]
    fn test_update_timers_tima_disabled() {
        use crate::io::{TAC, TIMA};
        let mut state = State::new();

        state.write(TIMA, 0x00);
        state.write(TAC, 0x00); // Timer disabled

        // Run cycles - TIMA should not change when disabled
        update_timers(&mut state, 1024);
        assert_eq!(state.read(TIMA), 0x00);
    }

    #[test]
    fn test_update_timers_tima_enabled() {
        use crate::io::{TAC, TIMA, TMA};
        let mut state = State::new();

        state.write(TIMA, 0x00);
        state.write(TMA, 0x00);
        state.write(TAC, 0x04); // Timer enabled, 4096 Hz (1024 cycles per increment)

        // Run 1024 cycles - should increment TIMA by 1
        update_timers(&mut state, 1024);
        assert_eq!(state.read(TIMA), 0x01);

        // Run 2048 more cycles - should increment TIMA by 2
        update_timers(&mut state, 2048);
        assert_eq!(state.read(TIMA), 0x03);
    }

    #[test]
    fn test_update_timers_overflow() {
        use crate::io::{IF, TAC, TIMA, TMA};
        let mut state = State::new();

        state.write(TIMA, 0xFF);
        state.write(TMA, 0x10);
        state.write(TAC, 0x04); // Timer enabled, 4096 Hz
        state.write(IF, 0x00);

        // Run 1024 cycles - should overflow and reload from TMA
        update_timers(&mut state, 1024);
        assert_eq!(state.read(TIMA), 0x10); // Reloaded from TMA

        // Check timer interrupt flag is set (bit 2)
        assert_eq!(state.read(IF) & 0x04, 0x04);
    }

    #[test]
    fn test_update_timers_fast_clock() {
        use crate::io::{TAC, TIMA};
        let mut state = State::new();

        state.write(TIMA, 0x00);
        state.write(TAC, 0x05); // Timer enabled, 262144 Hz (16 cycles per increment)

        // Run 64 cycles - should increment TIMA by 4
        update_timers(&mut state, 64);
        assert_eq!(state.read(TIMA), 0x04);
    }
}
