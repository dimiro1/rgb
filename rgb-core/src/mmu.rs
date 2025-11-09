/// Memory Management Unit (MMU) for Game Boy
///
/// Handles all memory access including:
/// - ROM banking (MBC1, MBC3, MBC5)
/// - RAM banking
/// - Memory-mapped I/O
/// - Video RAM (VRAM)
/// - Work RAM (WRAM)
/// - High RAM (HRAM)
/// - Object Attribute Memory (OAM)
use crate::cartridge::{Cartridge, CartridgeType};

/// Game Boy Memory Map:
/// 0x0000-0x3FFF : ROM Bank 0 (16KB) - Fixed
/// 0x4000-0x7FFF : ROM Bank 1-N (16KB) - Switchable
/// 0x8000-0x9FFF : VRAM (8KB)
/// 0xA000-0xBFFF : External RAM (8KB) - Switchable
/// 0xC000-0xDFFF : Work RAM (8KB)
/// 0xE000-0xFDFF : Echo RAM (mirror of 0xC000-0xDDFF)
/// 0xFE00-0xFE9F : OAM - Sprite Attribute Table
/// 0xFEA0-0xFEFF : Prohibited
/// 0xFF00-0xFF7F : I/O Registers
/// 0xFF80-0xFFFE : High RAM (127 bytes)
/// 0xFFFF        : Interrupt Enable Register
pub struct Mmu {
    /// Cartridge (contains ROM)
    pub cartridge: Cartridge,

    /// Current ROM bank (for 0x4000-0x7FFF region)
    rom_bank: usize,

    /// Current RAM bank (for 0xA000-0xBFFF region)
    ram_bank: usize,

    /// External RAM enabled flag
    ram_enabled: bool,

    /// External RAM (if cartridge has RAM)
    external_ram: Vec<u8>,

    /// Video RAM (8KB)
    vram: [u8; 0x2000],

    /// Work RAM (8KB)
    wram: [u8; 0x2000],

    /// High RAM (127 bytes)
    hram: [u8; 0x7F],

    /// Object Attribute Memory - Sprites (160 bytes)
    oam: [u8; 0xA0],

    /// I/O Registers (128 bytes)
    io: [u8; 0x80],

    /// MBC1 specific: Banking mode (0 = ROM banking, 1 = RAM banking)
    mbc1_mode: u8,

    /// MBC1 specific: Upper bank bits (can be used for ROM or RAM banking)
    mbc1_upper_bits: u8,
}

impl Mmu {
    /// Create a new MMU with the given cartridge
    pub fn new(cartridge: Cartridge) -> Self {
        // Allocate external RAM based on cartridge header
        let ram_size = cartridge.header.ram_size;
        let external_ram = vec![0; ram_size];

        Mmu {
            cartridge,
            rom_bank: 1, // Start with bank 1 for 0x4000-0x7FFF
            ram_bank: 0,
            ram_enabled: false,
            external_ram,
            vram: [0; 0x2000],
            wram: [0; 0x2000],
            hram: [0; 0x7F],
            oam: [0; 0xA0],
            io: [0; 0x80],
            mbc1_mode: 0,
            mbc1_upper_bits: 0,
        }
    }

    /// Read a byte from memory
    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            // ROM Bank 0 (fixed)
            0x0000..=0x3FFF => self.cartridge.read(addr),

            // ROM Bank 1-N (switchable)
            0x4000..=0x7FFF => {
                let offset = (self.rom_bank * 0x4000) + (addr as usize - 0x4000);
                self.cartridge.rom.get(offset).copied().unwrap_or(0xFF)
            }

            // Video RAM
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize],

            // External RAM (cartridge RAM, switchable)
            0xA000..=0xBFFF => {
                if self.ram_enabled && !self.external_ram.is_empty() {
                    let offset = (self.ram_bank * 0x2000) + (addr as usize - 0xA000);
                    self.external_ram.get(offset).copied().unwrap_or(0xFF)
                } else {
                    0xFF
                }
            }

            // Work RAM
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize],

            // Echo RAM (mirrors 0xC000-0xDDFF)
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize],

            // Object Attribute Memory (OAM) - Sprites
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize],

            // Prohibited area
            0xFEA0..=0xFEFF => 0xFF,

            // I/O Registers
            0xFF00..=0xFF7F => self.io[(addr - 0xFF00) as usize],

            // High RAM
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],

            // Interrupt Enable Register
            0xFFFF => self.io[0x7F],
        }
    }

    /// Write a byte to memory
    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            // ROM area (MBC control registers)
            0x0000..=0x7FFF => self.mbc_write(addr, value),

            // Video RAM
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize] = value,

            // External RAM (cartridge RAM)
            0xA000..=0xBFFF => {
                if self.ram_enabled && !self.external_ram.is_empty() {
                    let offset = (self.ram_bank * 0x2000) + (addr as usize - 0xA000);
                    if offset < self.external_ram.len() {
                        self.external_ram[offset] = value;
                    }
                }
            }

            // Work RAM
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize] = value,

            // Echo RAM (writes to WRAM)
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize] = value,

            // OAM
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize] = value,

            // Prohibited area (ignored)
            0xFEA0..=0xFEFF => {}

            // I/O Registers
            0xFF00..=0xFF7F => self.io[(addr - 0xFF00) as usize] = value,

            // High RAM
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = value,

            // Interrupt Enable Register
            0xFFFF => self.io[0x7F] = value,
        }
    }

    /// Handle writes to ROM area (MBC banking control)
    fn mbc_write(&mut self, addr: u16, value: u8) {
        match self.cartridge.header.cartridge_type {
            CartridgeType::RomOnly => {
                // No banking, writes to ROM area are ignored (ROM is read-only)
            }

            CartridgeType::Mbc1 | CartridgeType::Mbc1Ram => {
                self.mbc1_write(addr, value);
            }

            CartridgeType::Mbc3 | CartridgeType::Mbc3Ram => {
                self.mbc3_write(addr, value);
            }

            CartridgeType::Mbc5 | CartridgeType::Mbc5Ram => {
                self.mbc5_write(addr, value);
            }

            _ => {
                // Unsupported MBC types - ignore writes
            }
        }
    }

    /// MBC1 banking control
    fn mbc1_write(&mut self, addr: u16, value: u8) {
        match addr {
            // 0x0000-0x1FFF: RAM Enable
            0x0000..=0x1FFF => {
                self.ram_enabled = (value & 0x0F) == 0x0A;
            }

            // 0x2000-0x3FFF: ROM Bank Number (lower 5 bits)
            0x2000..=0x3FFF => {
                let mut bank = (value & 0x1F) as usize;
                // Bank 0 is not accessible in switchable region, map to bank 1
                if bank == 0 {
                    bank = 1;
                }

                // Combine with upper bits if in ROM banking mode
                if self.mbc1_mode == 0 {
                    bank |= (self.mbc1_upper_bits as usize) << 5;
                }

                // Ensure bank is within ROM size
                let max_banks = self.cartridge.rom.len() / 0x4000;
                self.rom_bank = bank % max_banks;
            }

            // 0x4000-0x5FFF: RAM Bank Number or Upper ROM Bank bits
            0x4000..=0x5FFF => {
                self.mbc1_upper_bits = value & 0x03;

                if self.mbc1_mode == 0 {
                    // ROM banking mode: upper bits affect ROM bank
                    let lower_bits = self.rom_bank & 0x1F;
                    let mut bank = lower_bits | ((self.mbc1_upper_bits as usize) << 5);
                    if bank == 0 {
                        bank = 1;
                    }
                    let max_banks = self.cartridge.rom.len() / 0x4000;
                    self.rom_bank = bank % max_banks;
                } else {
                    // RAM banking mode: upper bits affect RAM bank
                    self.ram_bank = (self.mbc1_upper_bits & 0x03) as usize;
                }
            }

            // 0x6000-0x7FFF: Banking Mode Select
            0x6000..=0x7FFF => {
                self.mbc1_mode = value & 0x01;

                if self.mbc1_mode == 0 {
                    // Switched to ROM banking mode
                    self.ram_bank = 0;
                } else {
                    // Switched to RAM banking mode
                    // Keep upper 2 bits of ROM bank only
                    self.rom_bank &= 0x1F;
                    if self.rom_bank == 0 {
                        self.rom_bank = 1;
                    }
                }
            }

            _ => unreachable!(),
        }
    }

    /// MBC3 banking control
    fn mbc3_write(&mut self, addr: u16, value: u8) {
        match addr {
            // 0x0000-0x1FFF: RAM Enable
            0x0000..=0x1FFF => {
                self.ram_enabled = (value & 0x0F) == 0x0A;
            }

            // 0x2000-0x3FFF: ROM Bank Number (7 bits)
            0x2000..=0x3FFF => {
                let mut bank = (value & 0x7F) as usize;
                if bank == 0 {
                    bank = 1;
                }
                let max_banks = self.cartridge.rom.len() / 0x4000;
                self.rom_bank = bank % max_banks;
            }

            // 0x4000-0x5FFF: RAM Bank Number or RTC Register Select
            0x4000..=0x5FFF => {
                if value <= 0x03 {
                    // RAM bank
                    self.ram_bank = (value & 0x03) as usize;
                } else if value >= 0x08 && value <= 0x0C {
                    // RTC register (not implemented yet)
                    // TODO: RTC support
                }
            }

            // 0x6000-0x7FFF: Latch Clock Data (RTC)
            0x6000..=0x7FFF => {
                // TODO: RTC latch
            }

            _ => unreachable!(),
        }
    }

    /// MBC5 banking control
    fn mbc5_write(&mut self, addr: u16, value: u8) {
        match addr {
            // 0x0000-0x1FFF: RAM Enable
            0x0000..=0x1FFF => {
                self.ram_enabled = (value & 0x0F) == 0x0A;
            }

            // 0x2000-0x2FFF: ROM Bank Number (lower 8 bits)
            0x2000..=0x2FFF => {
                let lower = value as usize;
                let upper = (self.rom_bank >> 8) & 0x01;
                self.rom_bank = (upper << 8) | lower;

                let max_banks = self.cartridge.rom.len() / 0x4000;
                if self.rom_bank >= max_banks {
                    self.rom_bank %= max_banks;
                }
            }

            // 0x3000-0x3FFF: ROM Bank Number (9th bit)
            0x3000..=0x3FFF => {
                let lower = self.rom_bank & 0xFF;
                let upper = (value as usize) & 0x01;
                self.rom_bank = (upper << 8) | lower;

                let max_banks = self.cartridge.rom.len() / 0x4000;
                if self.rom_bank >= max_banks {
                    self.rom_bank %= max_banks;
                }
            }

            // 0x4000-0x5FFF: RAM Bank Number (4 bits)
            0x4000..=0x5FFF => {
                self.ram_bank = (value & 0x0F) as usize;
            }

            // 0x6000-0x7FFF: Unused
            0x6000..=0x7FFF => {}

            _ => unreachable!(),
        }
    }

    /// Get reference to VRAM for PPU rendering
    pub fn vram(&self) -> &[u8] {
        &self.vram
    }

    /// Get reference to OAM for PPU rendering
    pub fn oam(&self) -> &[u8] {
        &self.oam
    }
}

/// Implement Memory trait for Mmu
impl crate::memory::Memory for Mmu {
    fn read(&self, addr: u16) -> u8 {
        self.read(addr)
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.write(addr, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test cartridge with ROM ONLY
    fn create_test_cartridge(rom_size: usize) -> Cartridge {
        let mut rom = vec![0; rom_size];
        rom[0x0147] = 0x00; // ROM ONLY
        rom[0x0148] = if rom_size == 32 * 1024 {
            0x00
        } else if rom_size == 64 * 1024 {
            0x01
        } else if rom_size == 128 * 1024 {
            0x02
        } else {
            0x00
        };
        rom[0x0149] = 0x00; // No RAM

        // Calculate header checksum
        let mut checksum: u8 = 0;
        for &byte in &rom[0x0134..=0x014C] {
            checksum = checksum.wrapping_sub(byte).wrapping_sub(1);
        }
        rom[0x014D] = checksum;

        Cartridge::from_bytes(rom).unwrap()
    }

    #[test]
    fn test_mmu_rom_bank_0() {
        let cart = create_test_cartridge(32 * 1024);
        let mmu = Mmu::new(cart);

        // Read from ROM bank 0
        assert_eq!(mmu.read(0x0000), 0x00);
        assert_eq!(mmu.read(0x3FFF), 0x00);
    }

    #[test]
    fn test_mmu_wram_read_write() {
        let cart = create_test_cartridge(32 * 1024);
        let mut mmu = Mmu::new(cart);

        // Write to WRAM
        mmu.write(0xC000, 0xAB);
        mmu.write(0xDFFF, 0xCD);

        // Read back
        assert_eq!(mmu.read(0xC000), 0xAB);
        assert_eq!(mmu.read(0xDFFF), 0xCD);
    }

    #[test]
    fn test_mmu_echo_ram() {
        let cart = create_test_cartridge(32 * 1024);
        let mut mmu = Mmu::new(cart);

        // Write to WRAM
        mmu.write(0xC100, 0x42);

        // Read from echo RAM (should mirror WRAM)
        assert_eq!(mmu.read(0xE100), 0x42);

        // Write to echo RAM
        mmu.write(0xE200, 0x99);

        // Should be visible in WRAM
        assert_eq!(mmu.read(0xC200), 0x99);
    }

    #[test]
    fn test_mmu_hram() {
        let cart = create_test_cartridge(32 * 1024);
        let mut mmu = Mmu::new(cart);

        mmu.write(0xFF80, 0x11);
        mmu.write(0xFFFE, 0x22);

        assert_eq!(mmu.read(0xFF80), 0x11);
        assert_eq!(mmu.read(0xFFFE), 0x22);
    }

    #[test]
    fn test_mmu_prohibited_area() {
        let cart = create_test_cartridge(32 * 1024);
        let mut mmu = Mmu::new(cart);

        // Writes to prohibited area should be ignored
        mmu.write(0xFEA0, 0xFF);

        // Reads should return 0xFF
        assert_eq!(mmu.read(0xFEA0), 0xFF);
        assert_eq!(mmu.read(0xFEFF), 0xFF);
    }

    #[test]
    fn test_mmu_interrupt_enable() {
        let cart = create_test_cartridge(32 * 1024);
        let mut mmu = Mmu::new(cart);

        mmu.write(0xFFFF, 0x1F);
        assert_eq!(mmu.read(0xFFFF), 0x1F);
    }
}
