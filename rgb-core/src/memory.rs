/// Memory trait for Game Boy memory access
///
/// This trait allows for different memory implementations:
/// - Real MMU with banking for production
/// - Flat memory array for testing
pub trait Memory {
    /// Read a byte from memory
    fn read(&self, addr: u16) -> u8;

    /// Write a byte to memory
    fn write(&mut self, addr: u16, value: u8);
}

/// Simple flat memory implementation for testing
///
/// This provides a straightforward 64KB memory array with no banking,
/// ROM protection, or other complexities. Perfect for unit tests.
pub struct FlatMemory {
    mem: Box<[u8; 0x10000]>,
}

impl FlatMemory {
    /// Create a new flat memory with all bytes initialized to 0
    pub fn new() -> Self {
        FlatMemory {
            mem: Box::new([0; 0x10000]),
        }
    }
}

impl Default for FlatMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl Memory for FlatMemory {
    fn read(&self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.mem[addr as usize] = value;
    }
}
