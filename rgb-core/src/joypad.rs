/// Joypad input handler for Game Boy
///
/// P1 Register (0xFF00) layout:
/// Bit 7-6: Not used
/// Bit 5: P15 - Select Action buttons (0=select)
/// Bit 4: P14 - Select Direction buttons (0=select)
/// Bit 3: P13 - Down or Start (0=pressed)
/// Bit 2: P12 - Up or Select (0=pressed)
/// Bit 1: P11 - Left or B (0=pressed)
/// Bit 0: P10 - Right or A (0=pressed)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Right,
    Left,
    Up,
    Down,
    A,
    B,
    Select,
    Start,
}

pub struct Joypad {
    select_action: bool,
    select_direction: bool,
    right: bool,
    left: bool,
    up: bool,
    down: bool,
    a: bool,
    b: bool,
    select: bool,
    start: bool,
}

impl Joypad {
    pub fn new() -> Self {
        Self {
            select_action: true,
            select_direction: true,
            right: false,
            left: false,
            up: false,
            down: false,
            a: false,
            b: false,
            select: false,
            start: false,
        }
    }

    pub fn press(&mut self, button: Button) {
        match button {
            Button::Right => self.right = true,
            Button::Left => self.left = true,
            Button::Up => self.up = true,
            Button::Down => self.down = true,
            Button::A => self.a = true,
            Button::B => self.b = true,
            Button::Select => self.select = true,
            Button::Start => self.start = true,
        }
    }

    pub fn release(&mut self, button: Button) {
        match button {
            Button::Right => self.right = false,
            Button::Left => self.left = false,
            Button::Up => self.up = false,
            Button::Down => self.down = false,
            Button::A => self.a = false,
            Button::B => self.b = false,
            Button::Select => self.select = false,
            Button::Start => self.start = false,
        }
    }

    pub fn read(&self) -> u8 {
        let mut value = 0xCF;

        if !self.select_direction {
            value |= 0x10;
        }

        if !self.select_action {
            value |= 0x20;
        }

        if self.select_direction {
            if self.down { value &= !0x08; }
            if self.up { value &= !0x04; }
            if self.left { value &= !0x02; }
            if self.right { value &= !0x01; }
        }

        if self.select_action {
            if self.start { value &= !0x08; }
            if self.select { value &= !0x04; }
            if self.b { value &= !0x02; }
            if self.a { value &= !0x01; }
        }

        value
    }

    pub fn write(&mut self, value: u8) {
        self.select_action = (value & 0x20) == 0;
        self.select_direction = (value & 0x10) == 0;
    }
}

impl Default for Joypad {
    fn default() -> Self {
        Self::new()
    }
}
