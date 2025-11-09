use rgb_core::{
    cartridge::Cartridge,
    joypad::Button,
    ppu::{Framebuffer, SCREEN_HEIGHT, SCREEN_WIDTH},
    system::GameBoy,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen::Clamped;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

/// Set up panic hook for better error messages in the browser console
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

/// Game Boy Emulator for WASM
#[wasm_bindgen]
pub struct Emulator {
    gameboy: Option<GameBoy>,
    running: bool,
    ctx: CanvasRenderingContext2d,
    scale: u32,
}

#[wasm_bindgen]
impl Emulator {
    /// Create a new emulator instance
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: &str, scale: u32) -> Result<Emulator, JsValue> {
        let window = web_sys::window().ok_or("No window object")?;
        let document = window.document().ok_or("No document object")?;
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or(format!("Canvas '{}' not found", canvas_id))?
            .dyn_into::<HtmlCanvasElement>()?;

        // Set canvas size
        canvas.set_width(SCREEN_WIDTH as u32 * scale);
        canvas.set_height(SCREEN_HEIGHT as u32 * scale);

        let ctx = canvas
            .get_context("2d")?
            .ok_or("No 2d context")?
            .dyn_into::<CanvasRenderingContext2d>()?;

        // Disable image smoothing for crisp pixels
        ctx.set_image_smoothing_enabled(false);

        Ok(Emulator {
            gameboy: None,
            running: false,
            ctx,
            scale,
        })
    }

    /// Load a ROM from bytes
    pub fn load_rom(&mut self, rom_data: &[u8]) -> Result<(), JsValue> {
        let cartridge = Cartridge::from_bytes(rom_data.to_vec())
            .map_err(|e| JsValue::from_str(&format!("Failed to load ROM: {}", e)))?;

        self.gameboy = Some(GameBoy::with_cartridge(cartridge));
        self.running = false;

        Ok(())
    }

    /// Check if a ROM is loaded
    pub fn is_loaded(&self) -> bool {
        self.gameboy.is_some()
    }

    /// Check if emulator is running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Start emulation
    pub fn start(&mut self) {
        if self.gameboy.is_some() {
            self.running = true;
        }
    }

    /// Stop emulation
    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn reset(&mut self) -> Result<(), JsValue> {
        if self.gameboy.is_some() {
            self.running = false;
            Ok(())
        } else {
            Err(JsValue::from_str("No ROM loaded"))
        }
    }

    /// Step the emulator for one frame (approximately 70224 cycles)
    /// Returns true when complete
    pub fn step_frame(&mut self) -> Result<bool, JsValue> {
        if let Some(ref mut gameboy) = self.gameboy {
            // Run for one full frame (154 scanlines * 456 dots per scanline)
            const CYCLES_PER_FRAME: u64 = 70224;
            let start_cycles = gameboy.cycles;

            // Execute instructions until we've completed a full frame
            while gameboy.cycles - start_cycles < CYCLES_PER_FRAME {
                gameboy.step_with_ppu();
            }

            Ok(true)
        } else {
            Err(JsValue::from_str("No ROM loaded"))
        }
    }

    /// Render the screen to the canvas
    pub fn render(&mut self) -> Result<(), JsValue> {
        if let Some(ref gameboy) = self.gameboy {
            let framebuffer = gameboy.ppu.framebuffer();
            self.render_framebuffer(framebuffer)?;
        }
        Ok(())
    }

    /// Private helper to render framebuffer to canvas
    fn render_framebuffer(&self, framebuffer: &Framebuffer) -> Result<(), JsValue> {
        // Convert 2-bit grayscale palette to RGBA
        // Game Boy colors: 0 = lightest (white), 3 = darkest (black)
        const COLORS: [[u8; 3]; 4] = [
            [0x9B, 0xBC, 0x0F], // Lightest (greenish white)
            [0x8B, 0xAC, 0x0F], // Light
            [0x30, 0x62, 0x30], // Dark
            [0x0F, 0x38, 0x0F], // Darkest (greenish black)
        ];

        // Create scaled RGBA buffer
        let scaled_width = SCREEN_WIDTH * self.scale as usize;
        let scaled_height = SCREEN_HEIGHT * self.scale as usize;
        let mut rgba_data = vec![0u8; scaled_width * scaled_height * 4];

        // Scale up during conversion
        for y in 0..scaled_height {
            for x in 0..scaled_width {
                let src_y = y / self.scale as usize;
                let src_x = x / self.scale as usize;
                let pixel_value = framebuffer[src_y][src_x] as usize;
                let color = COLORS[pixel_value];
                let idx = (y * scaled_width + x) * 4;

                rgba_data[idx] = color[0]; // R
                rgba_data[idx + 1] = color[1]; // G
                rgba_data[idx + 2] = color[2]; // B
                rgba_data[idx + 3] = 255; // A (fully opaque)
            }
        }

        // Create ImageData and draw to canvas
        let image_data = ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&rgba_data),
            scaled_width as u32,
            scaled_height as u32,
        )?;

        self.ctx.put_image_data(&image_data, 0.0, 0.0)?;

        Ok(())
    }

    pub fn key_down(&mut self, button: u8) {
        if let Some(ref mut gameboy) = self.gameboy {
            let btn = match button {
                0 => Button::Right,
                1 => Button::Left,
                2 => Button::Up,
                3 => Button::Down,
                4 => Button::A,
                5 => Button::B,
                6 => Button::Select,
                7 => Button::Start,
                _ => return,
            };
            gameboy.joypad.press(btn);
        }
    }

    pub fn key_up(&mut self, button: u8) {
        if let Some(ref mut gameboy) = self.gameboy {
            let btn = match button {
                0 => Button::Right,
                1 => Button::Left,
                2 => Button::Up,
                3 => Button::Down,
                4 => Button::A,
                5 => Button::B,
                6 => Button::Select,
                7 => Button::Start,
                _ => return,
            };
            gameboy.joypad.release(btn);
        }
    }
}

/// Log a message to the browser console
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}

/// Helper macro for logging
#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => {
        $crate::log(&format!($($t)*))
    };
}
