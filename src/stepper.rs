//! Open-loop half-step control for a 28BYJ-48 through IN1..IN4.
use esp_hal::{delay::Delay, gpio::Output};

// Bits correspond to IN1, IN2, IN3, IN4 in that order.
const PHASES: [u8; 8] = [
    0b0001, 0b0011, 0b0010, 0b0110, 0b0100, 0b1100, 0b1000, 0b1001,
];
#[cfg(feature = "stepper-scan")]
pub const NOMINAL_STEPS_PER_REV: i32 = 4096;
#[cfg(feature = "stepper-scan")]
pub const SCAN_END: i32 = NOMINAL_STEPS_PER_REV / 2;
#[cfg(feature = "stepper-scan")]
pub const SCAN_INCREMENT: i32 = 128;

pub struct Stepper<'a> {
    outputs: [Output<'a>; 4],
    phase: usize,
    pub position: i32,
}

impl<'a> Stepper<'a> {
    pub fn new(outputs: [Output<'a>; 4]) -> Self {
        Self {
            outputs,
            phase: 0,
            position: 0,
        }
    }

    fn energize(&mut self) {
        for (index, output) in self.outputs.iter_mut().enumerate() {
            output.set_level(if PHASES[self.phase] & (1 << index) != 0 {
                esp_hal::gpio::Level::High
            } else {
                esp_hal::gpio::Level::Low
            });
        }
    }

    pub fn move_to(&mut self, target: i32, delay: &Delay) {
        if target == self.position {
            return;
        }
        // Establish the phase before counting subsequent transitions as steps.
        self.energize();
        delay.delay_millis(4);
        while self.position != target {
            if target > self.position {
                self.phase = (self.phase + 1) % PHASES.len();
                self.position += 1;
            } else {
                self.phase = (self.phase + PHASES.len() - 1) % PHASES.len();
                self.position -= 1;
            }
            self.energize();
            delay.delay_millis(4);
        }
    }

    pub fn release(&mut self) {
        for output in &mut self.outputs {
            output.set_low();
        }
    }

    #[cfg(feature = "stepper-scan")]
    pub fn angle_mdeg(&self) -> i32 {
        self.position * 360_000 / NOMINAL_STEPS_PER_REV
    }
}
