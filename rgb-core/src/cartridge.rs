/// Game Boy Cartridge module
///
/// This module handles loading and parsing Game Boy ROM files (.gb).
/// Supports original DMG (Game Boy) only - no CGB (Color Game Boy) support.
/// Focuses on the most common cartridge types: ROM ONLY, MBC1, MBC3, and MBC5.
///
/// Note: This implementation does not verify the Nintendo logo or use a BIOS,
/// as the system state is initialized directly to post-boot values.
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

/// Cartridge type indicating the Memory Bank Controller (MBC)
/// Only includes the most common types for this implementation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CartridgeType {
    RomOnly,
    Mbc1,
    Mbc1Ram,
    Mbc3,
    Mbc3Ram,
    Mbc5,
    Mbc5Ram,
    Unsupported(u8),
}

impl CartridgeType {
    /// Parse cartridge type from byte value at 0x0147
    /// Only recognizes common MBC types
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0x00 => CartridgeType::RomOnly,
            0x01 => CartridgeType::Mbc1,
            0x02 => CartridgeType::Mbc1Ram,
            0x03 => CartridgeType::Mbc1Ram, // MBC1+RAM+BATTERY (treat as MBC1+RAM)
            0x11 => CartridgeType::Mbc3,
            0x12 => CartridgeType::Mbc3Ram,
            0x13 => CartridgeType::Mbc3Ram, // MBC3+RAM+BATTERY (treat as MBC3+RAM)
            0x19 => CartridgeType::Mbc5,
            0x1A => CartridgeType::Mbc5Ram,
            0x1B => CartridgeType::Mbc5Ram, // MBC5+RAM+BATTERY (treat as MBC5+RAM)
            _ => CartridgeType::Unsupported(byte),
        }
    }

    /// Check if this cartridge type includes RAM
    pub fn has_ram(&self) -> bool {
        matches!(
            self,
            CartridgeType::Mbc1Ram | CartridgeType::Mbc3Ram | CartridgeType::Mbc5Ram
        )
    }
}

impl fmt::Display for CartridgeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CartridgeType::RomOnly => write!(f, "ROM ONLY"),
            CartridgeType::Mbc1 => write!(f, "MBC1"),
            CartridgeType::Mbc1Ram => write!(f, "MBC1+RAM"),
            CartridgeType::Mbc3 => write!(f, "MBC3"),
            CartridgeType::Mbc3Ram => write!(f, "MBC3+RAM"),
            CartridgeType::Mbc5 => write!(f, "MBC5"),
            CartridgeType::Mbc5Ram => write!(f, "MBC5+RAM"),
            CartridgeType::Unsupported(byte) => write!(f, "UNSUPPORTED (0x{:02X})", byte),
        }
    }
}

/// Game Boy cartridge header information
#[derive(Debug, Clone)]
pub struct CartridgeHeader {
    /// Game title (0x0134-0x0143)
    pub title: String,
    /// Cartridge type (0x0147)
    pub cartridge_type: CartridgeType,
    /// ROM size in bytes (0x0148)
    pub rom_size: usize,
    /// RAM size in bytes (0x0149)
    pub ram_size: usize,
    /// ROM version (0x014C)
    pub rom_version: u8,
    /// Header checksum (0x014D)
    pub header_checksum: u8,
}

impl CartridgeHeader {
    /// Parse cartridge header from ROM data
    ///
    /// # Arguments
    /// * `rom` - ROM data bytes (must be at least 0x0150 bytes)
    ///
    /// # Returns
    /// Result containing CartridgeHeader or error message
    pub fn parse(rom: &[u8]) -> Result<Self, String> {
        if rom.len() < 0x0150 {
            return Err(format!(
                "ROM too small: {} bytes (minimum 0x0150 required)",
                rom.len()
            ));
        }

        // Extract title (0x0134-0x0143, null-terminated or space-padded)
        let title_bytes = &rom[0x0134..=0x0143];
        let title = String::from_utf8_lossy(title_bytes)
            .trim_end_matches('\0')
            .trim()
            .to_string();

        // Cartridge type
        let cartridge_type = CartridgeType::from_byte(rom[0x0147]);

        // ROM size: typically 32 KiB × (1 << value)
        let rom_size_code = rom[0x0148];
        let rom_size = match rom_size_code {
            0x00 => 32 * 1024,       // 32 KiB (no banking)
            0x01 => 64 * 1024,       // 64 KiB (4 banks)
            0x02 => 128 * 1024,      // 128 KiB (8 banks)
            0x03 => 256 * 1024,      // 256 KiB (16 banks)
            0x04 => 512 * 1024,      // 512 KiB (32 banks)
            0x05 => 1024 * 1024,     // 1 MiB (64 banks)
            0x06 => 2 * 1024 * 1024, // 2 MiB (128 banks)
            0x07 => 4 * 1024 * 1024, // 4 MiB (256 banks)
            0x08 => 8 * 1024 * 1024, // 8 MiB (512 banks)
            _ => {
                return Err(format!("Unknown ROM size code: 0x{:02X}", rom_size_code));
            }
        };

        // RAM size
        let ram_size_code = rom[0x0149];
        let ram_size = match ram_size_code {
            0x00 => 0,          // No RAM
            0x01 => 2 * 1024,   // 2 KiB (unused in practice)
            0x02 => 8 * 1024,   // 8 KiB (1 bank)
            0x03 => 32 * 1024,  // 32 KiB (4 banks of 8 KiB)
            0x04 => 128 * 1024, // 128 KiB (16 banks of 8 KiB)
            0x05 => 64 * 1024,  // 64 KiB (8 banks of 8 KiB)
            _ => {
                return Err(format!("Unknown RAM size code: 0x{:02X}", ram_size_code));
            }
        };

        // Other header fields
        let rom_version = rom[0x014C];
        let header_checksum = rom[0x014D];

        // Verify header checksum
        let mut checksum: u8 = 0;
        for &byte in &rom[0x0134..=0x014C] {
            checksum = checksum.wrapping_sub(byte).wrapping_sub(1);
        }

        if checksum != header_checksum {
            return Err(format!(
                "Header checksum mismatch: calculated 0x{:02X}, expected 0x{:02X}",
                checksum, header_checksum
            ));
        }

        Ok(CartridgeHeader {
            title,
            cartridge_type,
            rom_size,
            ram_size,
            rom_version,
            header_checksum,
        })
    }
}

impl fmt::Display for CartridgeHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== Cartridge Information ===")?;
        writeln!(f, "Title: {}", self.title)?;
        writeln!(f, "Cartridge Type: {}", self.cartridge_type)?;
        writeln!(f, "ROM Size: {} KiB", self.rom_size / 1024)?;
        writeln!(f, "RAM Size: {} KiB", self.ram_size / 1024)?;
        writeln!(f, "Version: {}", self.rom_version)?;
        Ok(())
    }
}

/// Represents a loaded Game Boy cartridge
#[derive(Debug, Clone)]
pub struct Cartridge {
    /// Cartridge header information
    pub header: CartridgeHeader,
    /// ROM data
    pub rom: Vec<u8>,
}

impl Cartridge {
    /// Load a cartridge from a file
    ///
    /// # Arguments
    /// * `path` - Path to the ROM file (.gb)
    ///
    /// # Returns
    /// Result containing Cartridge or IO error
    pub fn load<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let rom = fs::read(path)?;
        Self::from_bytes(rom)
    }

    /// Create a cartridge from ROM bytes
    ///
    /// # Arguments
    /// * `rom` - ROM data bytes
    ///
    /// # Returns
    /// Result containing Cartridge or IO error
    pub fn from_bytes(rom: Vec<u8>) -> io::Result<Self> {
        let header = CartridgeHeader::parse(&rom)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(Cartridge { header, rom })
    }

    /// Read a byte from ROM at the specified address
    ///
    /// # Arguments
    /// * `addr` - Address to read from (0x0000-0xFFFF)
    ///
    /// # Returns
    /// Byte value or 0xFF if address is out of bounds
    pub fn read(&self, addr: u16) -> u8 {
        let addr = addr as usize;
        if addr < self.rom.len() {
            self.rom[addr]
        } else {
            0xFF // Return 0xFF for unmapped addresses
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cartridge_type_from_byte() {
        assert_eq!(CartridgeType::from_byte(0x00), CartridgeType::RomOnly);
        assert_eq!(CartridgeType::from_byte(0x01), CartridgeType::Mbc1);
        assert_eq!(CartridgeType::from_byte(0x02), CartridgeType::Mbc1Ram);
        assert_eq!(CartridgeType::from_byte(0x03), CartridgeType::Mbc1Ram); // Battery treated as RAM
        assert_eq!(CartridgeType::from_byte(0x11), CartridgeType::Mbc3);
        assert_eq!(CartridgeType::from_byte(0x13), CartridgeType::Mbc3Ram);
        assert_eq!(CartridgeType::from_byte(0x19), CartridgeType::Mbc5);
        assert_eq!(CartridgeType::from_byte(0x1B), CartridgeType::Mbc5Ram);

        if let CartridgeType::Unsupported(0xFD) = CartridgeType::from_byte(0xFD) {
            // Correct - TAMA5 is unsupported
        } else {
            panic!("Expected Unsupported variant");
        }
    }

    #[test]
    fn test_cartridge_type_has_ram() {
        assert!(!CartridgeType::RomOnly.has_ram());
        assert!(!CartridgeType::Mbc1.has_ram());
        assert!(CartridgeType::Mbc1Ram.has_ram());
        assert!(!CartridgeType::Mbc3.has_ram());
        assert!(CartridgeType::Mbc3Ram.has_ram());
        assert!(!CartridgeType::Mbc5.has_ram());
        assert!(CartridgeType::Mbc5Ram.has_ram());
    }

    #[test]
    fn test_parse_header_too_small() {
        let rom = vec![0; 0x100];
        let result = CartridgeHeader::parse(&rom);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too small"));
    }

    #[test]
    fn test_parse_header_checksum_mismatch() {
        let mut rom = vec![0; 0x0150];
        rom[0x0147] = 0x00; // ROM ONLY
        rom[0x0148] = 0x00; // 32 KiB
        rom[0x0149] = 0x00; // No RAM
        rom[0x014D] = 0xFF; // Invalid checksum

        let result = CartridgeHeader::parse(&rom);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("checksum mismatch"));
    }

    #[test]
    fn test_parse_header_valid() {
        let mut rom = vec![0; 0x8000]; // 32 KiB ROM

        // Set title
        rom[0x0134..0x013C].copy_from_slice(b"TESTGAME");

        // Set cartridge type (MBC1+RAM)
        rom[0x0147] = 0x02;

        // Set ROM size (32 KiB)
        rom[0x0148] = 0x00;

        // Set RAM size (8 KiB)
        rom[0x0149] = 0x02;

        // Calculate and set header checksum
        let mut checksum: u8 = 0;
        for &byte in &rom[0x0134..=0x014C] {
            checksum = checksum.wrapping_sub(byte).wrapping_sub(1);
        }
        rom[0x014D] = checksum;

        let header = CartridgeHeader::parse(&rom).unwrap();

        assert_eq!(header.title, "TESTGAME");
        assert_eq!(header.cartridge_type, CartridgeType::Mbc1Ram);
        assert_eq!(header.rom_size, 32 * 1024);
        assert_eq!(header.ram_size, 8 * 1024);
    }

    #[test]
    fn test_parse_header_checksum_fail() {
        let mut rom = vec![0; 0x8000];
        rom[0x0147] = 0x00;
        rom[0x0148] = 0x00;
        rom[0x0149] = 0x00;
        rom[0x014D] = 0xFF; // Wrong checksum

        let result = CartridgeHeader::parse(&rom);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("checksum mismatch"));
    }

    #[test]
    fn test_cartridge_read() {
        let mut rom = vec![0; 0x8000];
        rom[0x0147] = 0x00; // ROM ONLY
        rom[0x0148] = 0x00; // 32 KiB
        rom[0x0149] = 0x00; // No RAM

        // Calculate checksum
        let mut checksum: u8 = 0;
        for &byte in &rom[0x0134..=0x014C] {
            checksum = checksum.wrapping_sub(byte).wrapping_sub(1);
        }
        rom[0x014D] = checksum;

        // Set some test data
        rom[0x0100] = 0xAB;
        rom[0x1000] = 0xCD;

        let cartridge = Cartridge::from_bytes(rom).unwrap();

        assert_eq!(cartridge.read(0x0100), 0xAB);
        assert_eq!(cartridge.read(0x1000), 0xCD);
        assert_eq!(cartridge.read(0xFFFF), 0xFF); // Out of bounds
    }

    #[test]
    fn test_rom_size_calculations() {
        let sizes = vec![
            (0x00, 32 * 1024),
            (0x01, 64 * 1024),
            (0x02, 128 * 1024),
            (0x03, 256 * 1024),
            (0x04, 512 * 1024),
            (0x05, 1024 * 1024),
            (0x06, 2 * 1024 * 1024),
            (0x07, 4 * 1024 * 1024),
        ];

        for (code, expected_size) in sizes {
            let mut rom = vec![0; expected_size];
            rom[0x0147] = 0x00;
            rom[0x0148] = code;
            rom[0x0149] = 0x00;

            let mut checksum: u8 = 0;
            for &byte in &rom[0x0134..=0x014C] {
                checksum = checksum.wrapping_sub(byte).wrapping_sub(1);
            }
            rom[0x014D] = checksum;

            let header = CartridgeHeader::parse(&rom).unwrap();
            assert_eq!(header.rom_size, expected_size);
        }
    }

    #[test]
    fn test_ram_size_calculations() {
        let sizes = vec![
            (0x00, 0),
            (0x01, 2 * 1024),
            (0x02, 8 * 1024),
            (0x03, 32 * 1024),
            (0x04, 128 * 1024),
            (0x05, 64 * 1024),
        ];

        for (code, expected_size) in sizes {
            let mut rom = vec![0; 0x8000];
            rom[0x0147] = 0x00;
            rom[0x0148] = 0x00;
            rom[0x0149] = code;

            let mut checksum: u8 = 0;
            for &byte in &rom[0x0134..=0x014C] {
                checksum = checksum.wrapping_sub(byte).wrapping_sub(1);
            }
            rom[0x014D] = checksum;

            let header = CartridgeHeader::parse(&rom).unwrap();
            assert_eq!(header.ram_size, expected_size);
        }
    }
}
