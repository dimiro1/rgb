/// PPU (Picture Processing Unit) implementation

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    HBlank = 0,
    VBlank = 1,
    OamSearch = 2,
    PixelTransfer = 3,
}

pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;

pub type Framebuffer = [[u8; SCREEN_WIDTH]; SCREEN_HEIGHT];

pub struct Ppu {
    ly: u8,
    dots: u16,
    mode: Mode,
    lcdc: u8,
    stat: u8,
    scy: u8,
    scx: u8,
    lyc: u8,
    bgp: u8,
    obp0: u8,
    obp1: u8,
    wy: u8,
    wx: u8,
    framebuffer: Box<Framebuffer>,
    sprite_buffer: Vec<SpriteData>,
    pub vblank_interrupt: bool,
    pub stat_interrupt: bool,
    pub should_scan_oam: bool,
    pub should_render_scanline: bool,
}

#[derive(Debug, Clone, Copy)]
struct SpriteData {
    y: i16,
    x: i16,
    tile_index: u8,
    attributes: u8,
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            ly: 0,
            dots: 252,
            mode: Mode::HBlank,
            lcdc: 0x91,
            stat: Mode::HBlank as u8,
            scy: 0,
            scx: 0,
            lyc: 0,
            bgp: 0xFC,
            obp0: 0xFF,
            obp1: 0xFF,
            wy: 0,
            wx: 0,
            framebuffer: Box::new([[0; SCREEN_WIDTH]; SCREEN_HEIGHT]),
            sprite_buffer: Vec::with_capacity(10),
            vblank_interrupt: false,
            stat_interrupt: false,
            should_scan_oam: false,
            should_render_scanline: false,
        }
    }

    pub fn step(&mut self, cycles: u64) {
        if !self.is_lcd_enabled() {
            return;
        }

        for _ in 0..cycles {
            self.dots += 1;

            match self.mode {
                Mode::OamSearch if self.dots == 80 => {
                    self.set_mode(Mode::PixelTransfer);
                }
                Mode::PixelTransfer if self.dots == 252 => {
                    self.should_render_scanline = true;
                    self.set_mode(Mode::HBlank);
                }
                Mode::HBlank if self.dots == 456 => {
                    self.dots = 0;
                    self.ly += 1;

                    if self.ly == 144 {
                        self.set_mode(Mode::VBlank);
                        self.vblank_interrupt = true;
                    } else {
                        self.should_scan_oam = true;
                        self.set_mode(Mode::OamSearch);
                    }

                    self.update_lyc_flag();
                }
                Mode::VBlank if self.dots == 456 => {
                    self.dots = 0;
                    self.ly += 1;

                    if self.ly == 154 {
                        self.ly = 0;
                        self.should_scan_oam = true;
                        self.set_mode(Mode::OamSearch);
                    }

                    self.update_lyc_flag();
                }
                _ => {}
            }
        }
    }

    fn set_mode(&mut self, mode: Mode) {
        let old_mode = self.mode;
        self.mode = mode;
        self.stat = (self.stat & 0xFC) | (mode as u8);

        let stat_int = match mode {
            Mode::HBlank => self.stat & 0x08 != 0,
            Mode::VBlank => self.stat & 0x10 != 0,
            Mode::OamSearch => self.stat & 0x20 != 0,
            Mode::PixelTransfer => false,
        };

        if stat_int && old_mode != mode {
            self.stat_interrupt = true;
        }
    }

    fn update_lyc_flag(&mut self) {
        let lyc_eq_ly = self.ly == self.lyc;

        if lyc_eq_ly {
            self.stat |= 0x04;
        } else {
            self.stat &= !0x04;
        }

        if lyc_eq_ly && (self.stat & 0x40 != 0) {
            self.stat_interrupt = true;
        }
    }

    pub fn is_lcd_enabled(&self) -> bool {
        self.lcdc & 0x80 != 0
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn is_vblank(&self) -> bool {
        self.mode == Mode::VBlank
    }

    pub fn framebuffer(&self) -> &Framebuffer {
        &self.framebuffer
    }

    pub fn read_lcdc(&self) -> u8 {
        self.lcdc
    }

    pub fn write_lcdc(&mut self, value: u8) {
        let lcd_was_on = self.lcdc & 0x80 != 0;
        let lcd_now_on = value & 0x80 != 0;

        self.lcdc = value;

        if lcd_was_on && !lcd_now_on {
            self.ly = 0;
            self.dots = 0;
            self.mode = Mode::OamSearch;
            self.stat = (self.stat & 0xFC) | (Mode::OamSearch as u8);
        }
    }

    pub fn read_stat(&self) -> u8 {
        self.stat | 0x80
    }

    pub fn write_stat(&mut self, value: u8) {
        self.stat = (self.stat & 0x07) | (value & 0x78);
    }

    pub fn read_scy(&self) -> u8 {
        self.scy
    }

    pub fn write_scy(&mut self, value: u8) {
        self.scy = value;
    }

    pub fn read_scx(&self) -> u8 {
        self.scx
    }

    pub fn write_scx(&mut self, value: u8) {
        self.scx = value;
    }

    pub fn read_ly(&self) -> u8 {
        self.ly
    }

    pub fn read_lyc(&self) -> u8 {
        self.lyc
    }

    pub fn write_lyc(&mut self, value: u8) {
        self.lyc = value;
        self.update_lyc_flag();
    }

    pub fn read_bgp(&self) -> u8 {
        self.bgp
    }

    pub fn write_bgp(&mut self, value: u8) {
        self.bgp = value;
    }

    pub fn read_obp0(&self) -> u8 {
        self.obp0
    }

    pub fn write_obp0(&mut self, value: u8) {
        self.obp0 = value;
    }

    pub fn read_obp1(&self) -> u8 {
        self.obp1
    }

    pub fn write_obp1(&mut self, value: u8) {
        self.obp1 = value;
    }

    pub fn read_wy(&self) -> u8 {
        self.wy
    }

    pub fn write_wy(&mut self, value: u8) {
        self.wy = value;
    }

    pub fn read_wx(&self) -> u8 {
        self.wx
    }

    pub fn write_wx(&mut self, value: u8) {
        self.wx = value;
    }

    fn bg_window_enabled(&self) -> bool {
        self.lcdc & 0x01 != 0
    }

    fn sprites_enabled(&self) -> bool {
        self.lcdc & 0x02 != 0
    }

    fn sprite_size(&self) -> u8 {
        if self.lcdc & 0x04 != 0 {
            16 // 8x16
        } else {
            8 // 8x8
        }
    }

    fn bg_tile_map_area(&self) -> u16 {
        if self.lcdc & 0x08 != 0 {
            0x9C00
        } else {
            0x9800
        }
    }

    fn bg_window_tile_data_area(&self) -> (u16, bool) {
        if self.lcdc & 0x10 != 0 {
            (0x8000, false) // Unsigned addressing
        } else {
            (0x8800, true) // Signed addressing
        }
    }

    fn window_enabled(&self) -> bool {
        self.lcdc & 0x20 != 0
    }

    fn window_tile_map_area(&self) -> u16 {
        if self.lcdc & 0x40 != 0 {
            0x9C00
        } else {
            0x9800
        }
    }

    pub fn render_scanline(&mut self, vram: &[u8], oam: &[u8]) {
        if !self.is_lcd_enabled() || self.ly >= 144 {
            return;
        }

        let line = self.ly as usize;

        for x in 0..SCREEN_WIDTH {
            self.framebuffer[line][x] = 0;
        }

        if self.bg_window_enabled() {
            self.render_background(line, vram);
        }

        if self.window_enabled() && self.bg_window_enabled() {
            self.render_window(line, vram);
        }

        if self.sprites_enabled() {
            self.render_sprites(line, vram, oam);
        }
    }

    fn render_background(&mut self, line: usize, vram: &[u8]) {
        let (tile_data_base, is_signed) = self.bg_window_tile_data_area();
        let tile_map_base = self.bg_tile_map_area();

        let bg_y = self.scy.wrapping_add(line as u8) as usize;
        let tile_row = bg_y / 8;
        let tile_y_offset = bg_y % 8;

        for x in 0..SCREEN_WIDTH {
            let bg_x = self.scx.wrapping_add(x as u8) as usize;
            let tile_col = bg_x / 8;
            let tile_x_offset = bg_x % 8;

            let tile_map_addr = tile_map_base + ((tile_row % 32) * 32) as u16 + (tile_col % 32) as u16;
            let tile_index = vram[(tile_map_addr - 0x8000) as usize];

            let tile_addr = if is_signed {
                let offset = (tile_index as i8 as i16) * 16;
                (0x9000u16 as i16 + offset) as u16
            } else {
                tile_data_base + (tile_index as u16 * 16)
            };

            let color = self.get_tile_pixel(vram, tile_addr, tile_x_offset, tile_y_offset);
            let palette_color = self.apply_palette(color, self.bgp);

            self.framebuffer[line][x] = palette_color;
        }
    }

    fn render_window(&mut self, line: usize, vram: &[u8]) {
        if self.wy > line as u8 {
            return;
        }

        let window_x_start = if self.wx >= 7 { self.wx - 7 } else { 0 };

        if window_x_start >= 160 {
            return;
        }

        let (tile_data_base, is_signed) = self.bg_window_tile_data_area();
        let tile_map_base = self.window_tile_map_area();

        let window_y = (line as u8).wrapping_sub(self.wy) as usize;
        let tile_row = window_y / 8;
        let tile_y_offset = window_y % 8;

        for screen_x in window_x_start as usize..SCREEN_WIDTH {
            let window_x = screen_x - window_x_start as usize;
            let tile_col = window_x / 8;
            let tile_x_offset = window_x % 8;

            let tile_map_addr = tile_map_base + ((tile_row % 32) * 32) as u16 + (tile_col % 32) as u16;
            let tile_index = vram[(tile_map_addr - 0x8000) as usize];

            let tile_addr = if is_signed {
                let offset = (tile_index as i8 as i16) * 16;
                (0x9000u16 as i16 + offset) as u16
            } else {
                tile_data_base + (tile_index as u16 * 16)
            };

            let color = self.get_tile_pixel(vram, tile_addr, tile_x_offset, tile_y_offset);
            let palette_color = self.apply_palette(color, self.bgp);

            self.framebuffer[line][screen_x] = palette_color;
        }
    }

    pub fn scan_oam(&mut self, oam: &[u8]) {
        self.sprite_buffer.clear();

        let sprite_height = self.sprite_size();
        let line = self.ly as i16;

        for i in 0..40 {
            let oam_addr = i * 4;
            let y = oam[oam_addr] as i16 - 16;
            let x = oam[oam_addr + 1] as i16 - 8;
            let tile_index = oam[oam_addr + 2];
            let attributes = oam[oam_addr + 3];

            if line >= y && line < y + sprite_height as i16 {
                self.sprite_buffer.push(SpriteData {
                    y,
                    x,
                    tile_index,
                    attributes,
                });

                if self.sprite_buffer.len() >= 10 {
                    break;
                }
            }
        }
    }

    fn render_sprites(&mut self, line: usize, vram: &[u8], oam: &[u8]) {
        if self.sprite_buffer.is_empty() {
            self.scan_oam(oam);
        }

        let sprite_height = self.sprite_size();

        for sprite in self.sprite_buffer.iter().rev() {
            if sprite.x < -7 || sprite.x >= 160 {
                continue;
            }

            let mut y_offset = (line as i16 - sprite.y) as usize;

            let y_flip = sprite.attributes & 0x40 != 0;
            if y_flip {
                y_offset = (sprite_height as usize - 1) - y_offset;
            }

            let tile_index = if sprite_height == 16 {
                sprite.tile_index & 0xFE
            } else {
                sprite.tile_index
            };

            let tile_num = if y_offset >= 8 {
                tile_index + 1
            } else {
                tile_index
            };
            let tile_y_offset = y_offset % 8;

            let tile_addr = 0x8000 + (tile_num as u16 * 16);

            let palette = if sprite.attributes & 0x10 != 0 {
                self.obp1
            } else {
                self.obp0
            };

            let x_flip = sprite.attributes & 0x20 != 0;
            let bg_priority = sprite.attributes & 0x80 != 0;

            for pixel_x in 0..8 {
                let screen_x = sprite.x + pixel_x;
                if screen_x < 0 || screen_x >= 160 {
                    continue;
                }

                let tile_x_offset = if x_flip { 7 - pixel_x as usize } else { pixel_x as usize };
                let color = self.get_tile_pixel(vram, tile_addr, tile_x_offset, tile_y_offset);

                if color == 0 {
                    continue;
                }

                if bg_priority && self.framebuffer[line][screen_x as usize] != 0 {
                    continue;
                }

                let palette_color = self.apply_palette(color, palette);
                self.framebuffer[line][screen_x as usize] = palette_color;
            }
        }
    }

    fn get_tile_pixel(&self, vram: &[u8], tile_addr: u16, x: usize, y: usize) -> u8 {
        let row_addr = tile_addr + (y as u16 * 2);
        let low_byte = vram[(row_addr - 0x8000) as usize];
        let high_byte = vram[(row_addr - 0x8000 + 1) as usize];

        let bit_pos = 7 - x;
        let low_bit = (low_byte >> bit_pos) & 1;
        let high_bit = (high_byte >> bit_pos) & 1;

        (high_bit << 1) | low_bit
    }

    fn apply_palette(&self, color: u8, palette: u8) -> u8 {
        match color {
            0 => palette & 0x03,
            1 => (palette >> 2) & 0x03,
            2 => (palette >> 4) & 0x03,
            3 => (palette >> 6) & 0x03,
            _ => 0,
        }
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

    #[test]
    fn test_mode_transitions() {
        let mut ppu = Ppu::new();
        assert_eq!(ppu.mode(), Mode::HBlank);

        ppu.step(204);
        assert_eq!(ppu.read_ly(), 1);
        assert_eq!(ppu.mode(), Mode::OamSearch);

        ppu.step(80);
        assert_eq!(ppu.mode(), Mode::PixelTransfer);

        ppu.step(172);
        assert_eq!(ppu.mode(), Mode::HBlank);

        ppu.step(204);
        assert_eq!(ppu.read_ly(), 2);
        assert_eq!(ppu.mode(), Mode::OamSearch);
    }

    #[test]
    fn test_vblank_interrupt() {
        let mut ppu = Ppu::new();
        assert!(!ppu.vblank_interrupt);

        // Go to line 144 (VBlank start)
        ppu.step(456 * 144);
        assert_eq!(ppu.read_ly(), 144);
        assert_eq!(ppu.mode(), Mode::VBlank);
        assert!(ppu.vblank_interrupt);
    }

    #[test]
    fn test_stat_register() {
        let mut ppu = Ppu::new();

        assert_eq!(ppu.read_stat() & 0x03, Mode::HBlank as u8);

        ppu.step(204);
        assert_eq!(ppu.read_stat() & 0x03, Mode::OamSearch as u8);

        ppu.step(80);
        assert_eq!(ppu.read_stat() & 0x03, Mode::PixelTransfer as u8);

        ppu.step(172);
        assert_eq!(ppu.read_stat() & 0x03, Mode::HBlank as u8);
    }

    #[test]
    fn test_lyc_comparison() {
        let mut ppu = Ppu::new();
        ppu.write_lyc(5);

        // LYC flag should be clear when LY != LYC
        assert_eq!(ppu.read_stat() & 0x04, 0);

        // Go to line 5
        ppu.step(456 * 5);
        assert_eq!(ppu.read_ly(), 5);

        // LYC flag should be set when LY == LYC
        assert_eq!(ppu.read_stat() & 0x04, 0x04);
    }
}
