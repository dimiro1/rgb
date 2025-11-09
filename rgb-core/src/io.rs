//! Memory-mapped I/O register addresses

// I/O Registers
pub const P1: u16 = 0xFF00; // Joypad
// pub const SC: u16 = 0xFF02; // Serial transfer control
pub const DIV: u16 = 0xFF04; // Divider register
pub const TIMA: u16 = 0xFF05; // Timer counter
pub const TMA: u16 = 0xFF06; // Timer modulo
pub const TAC: u16 = 0xFF07; // Timer control
pub const IF: u16 = 0xFF0F; // Interrupt flag

// Sound registers
pub const NR_10: u16 = 0xFF10; // Channel 1 sweep
pub const NR_11: u16 = 0xFF11; // Channel 1 length timer & duty cycle
pub const NR_12: u16 = 0xFF12; // Channel 1 volume & envelope
pub const NR_14: u16 = 0xFF14; // Channel 1 period high & control
pub const NR_21: u16 = 0xFF16; // Channel 2 length timer & duty cycle
pub const NR_22: u16 = 0xFF17; // Channel 2 volume & envelope
pub const NR_24: u16 = 0xFF19; // Channel 2 period high & control
pub const NR_30: u16 = 0xFF1A; // Channel 3 DAC enable
pub const NR_31: u16 = 0xFF1B; // Channel 3 length timer
pub const NR_32: u16 = 0xFF1C; // Channel 3 output level
pub const NR_34: u16 = 0xFF1E; // Channel 3 period high & control
pub const NR_41: u16 = 0xFF20; // Channel 4 length timer
pub const NR_42: u16 = 0xFF21; // Channel 4 volume & envelope
pub const NR_43: u16 = 0xFF22; // Channel 4 frequency & randomness
pub const NR_44: u16 = 0xFF23; // Channel 4 control
pub const NR_50: u16 = 0xFF24; // Master volume & VIN panning
pub const NR_51: u16 = 0xFF25; // Sound panning
pub const NR_52: u16 = 0xFF26; // Sound on/off

// LCD registers
pub const LCDC: u16 = 0xFF40; // LCD control
pub const STAT: u16 = 0xFF41; // LCD status
pub const SCY: u16 = 0xFF42; // Scroll Y
pub const SCX: u16 = 0xFF43; // Scroll X
pub const LY: u16 = 0xFF44; // LCD Y coordinate
pub const LYC: u16 = 0xFF45; // LY compare
// pub const DMA: u16 = 0xFF46; // OAM DMA source address & start
pub const BGP: u16 = 0xFF47; // Background palette
pub const OBP0: u16 = 0xFF48; // Object palette 0
pub const OBP1: u16 = 0xFF49; // Object palette 1
pub const WY: u16 = 0xFF4A; // Window Y position
pub const WX: u16 = 0xFF4B; // Window X position

pub const IE: u16 = 0xFFFF; // Interrupt enable
