//! LCD1602 with the user's PCF8574T I2C backpack.
//! Common mapping: P0=RS, P1=RW, P2=E, P3=backlight, P4..7=D4..7.
use core::fmt::{self, Write};
use embedded_hal::i2c::I2c;
use esp_hal::delay::Delay;

pub struct Lcd {
    pub address: u8,
}

impl Lcd {
    pub const ADDRESS: u8 = 0x27;

    pub const fn new() -> Self {
        Self {
            address: Self::ADDRESS,
        }
    }

    fn nibble<B: I2c>(&self, bus: &mut B, delay: &Delay, value: u8) -> Result<(), B::Error> {
        let value = value | 0x08; // Backlight on, RW held low.
        bus.write(self.address, &[value])?;
        delay.delay_micros(2);
        bus.write(self.address, &[value | 0x04])?;
        delay.delay_micros(2);
        bus.write(self.address, &[value])?;
        delay.delay_micros(100);
        Ok(())
    }

    fn send<B: I2c>(
        &self,
        bus: &mut B,
        delay: &Delay,
        value: u8,
        data: bool,
    ) -> Result<(), B::Error> {
        let rs = u8::from(data);
        self.nibble(bus, delay, (value & 0xf0) | rs)?;
        self.nibble(bus, delay, (value << 4) | rs)?;
        Ok(())
    }

    pub fn init<B: I2c>(&self, bus: &mut B, delay: &Delay) -> Result<(), B::Error> {
        // Establish RS=RW=E=0 before the reset sequence, including after an MCU reset.
        bus.write(self.address, &[0x08])?;
        delay.delay_millis(50);
        for pause in [5000, 5000, 200] {
            self.nibble(bus, delay, 0x30)?;
            delay.delay_micros(pause);
        }
        self.nibble(bus, delay, 0x20)?;
        self.send(bus, delay, 0x28, false)?; // 4-bit, two lines, 5x8 font.
        self.send(bus, delay, 0x08, false)?; // Display off during setup.
        self.send(bus, delay, 0x01, false)?;
        delay.delay_millis(5);
        self.send(bus, delay, 0x06, false)?; // Cursor increments, no scrolling.
        self.send(bus, delay, 0x0c, false)
    }

    pub fn show<B: I2c>(
        &self,
        bus: &mut B,
        delay: &Delay,
        mm: Option<u32>,
    ) -> Result<(), B::Error> {
        // Rewrite padded lines without clearing the screen to avoid flicker/stale digits.
        let mut distance = Line::new();
        if let Some(mm) = mm {
            let _ = write!(distance, "Dist: {}.{} cm", mm / 10, mm % 10);
        } else {
            let _ = distance.write_str("No valid echo");
        }
        self.lines(bus, delay, *b"BatMetal sonar  ", distance.0)
    }

    pub fn test<B: I2c>(&self, bus: &mut B, delay: &Delay) -> Result<(), B::Error> {
        self.lines(bus, delay, *b"TEST LCD 1602   ", *b"0123456789ABCDEF")
    }

    fn lines<B: I2c>(
        &self,
        bus: &mut B,
        delay: &Delay,
        first: [u8; 16],
        second: [u8; 16],
    ) -> Result<(), B::Error> {
        for (command, line) in [(0x80, first), (0xc0, second)] {
            self.send(bus, delay, command, false)?;
            for byte in line {
                self.send(bus, delay, byte, true)?;
            }
        }
        Ok(())
    }
}

struct Line([u8; 16], usize);
impl Line {
    fn new() -> Self {
        Self([b' '; 16], 0)
    }
}
impl Write for Line {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for byte in text.bytes() {
            if self.1 == self.0.len() {
                return Err(fmt::Error);
            }
            self.0[self.1] = byte;
            self.1 += 1;
        }
        Ok(())
    }
}
