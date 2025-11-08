/// Simple PPU (Picture Processing Unit) implementation
///
/// Game Boy LCD timing:
/// - Each scanline takes 456 dots (cycles)
/// - Lines 0-143: Visible scanlines
/// - Lines 144-153: V-Blank period
/// - Total: 154 lines per frame

pub struct Ppu {
    /// Current scanline (0-153)
    pub ly: u8,
    /// Cycles accumulated for current scanline
    cycles: u64,
}

impl Ppu {
    pub fn new() -> Self {
        Self { ly: 0, cycles: 0 }
    }

    /// Update PPU state based on CPU cycles
    /// Each scanline takes 456 cycles
    pub fn step(&mut self, cycles: u64) {
        self.cycles += cycles;

        // Each scanline takes 456 cycles
        const CYCLES_PER_SCANLINE: u64 = 456;

        while self.cycles >= CYCLES_PER_SCANLINE {
            self.cycles -= CYCLES_PER_SCANLINE;
            self.ly = (self.ly + 1) % 154;
        }
    }

    /// Read LCD Y-coordinate register (0xFF44)
    pub fn read_ly(&self) -> u8 {
        self.ly
    }

    /// Check if we're in V-Blank period (lines 144-153)
    pub fn is_vblank(&self) -> bool {
        self.ly >= 144
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ppu_ly_increment() {
        let mut ppu = Ppu::new();
        assert_eq!(ppu.read_ly(), 0);

        // Step by one scanline (456 cycles)
        ppu.step(456);
        assert_eq!(ppu.read_ly(), 1);

        // Step by another scanline
        ppu.step(456);
        assert_eq!(ppu.read_ly(), 2);
    }

    #[test]
    fn test_ppu_ly_wraps_at_154() {
        let mut ppu = Ppu::new();

        // Step through all 154 lines
        ppu.step(456 * 154);
        assert_eq!(ppu.read_ly(), 0);
    }

    #[test]
    fn test_vblank_detection() {
        let mut ppu = Ppu::new();
        assert!(!ppu.is_vblank());

        // Go to line 143 (last visible line)
        ppu.step(456 * 143);
        assert!(!ppu.is_vblank());

        // Go to line 144 (first V-Blank line)
        ppu.step(456);
        assert!(ppu.is_vblank());
        assert_eq!(ppu.read_ly(), 144);

        // Go to line 153 (last V-Blank line)
        ppu.step(456 * 9);
        assert!(ppu.is_vblank());
        assert_eq!(ppu.read_ly(), 153);

        // Wrap back to line 0
        ppu.step(456);
        assert!(!ppu.is_vblank());
        assert_eq!(ppu.read_ly(), 0);
    }

    #[test]
    fn test_partial_cycles() {
        let mut ppu = Ppu::new();

        // Step by less than a scanline
        ppu.step(100);
        assert_eq!(ppu.read_ly(), 0);

        // Add enough to complete the scanline
        ppu.step(356);
        assert_eq!(ppu.read_ly(), 1);
    }
}
