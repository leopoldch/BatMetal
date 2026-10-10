//! Bounded HC-SR04 acquisition. ECHO must pass through a 3.3 V divider.
use esp_hal::{
    delay::Delay,
    gpio::{Input, Output},
    time::Instant,
};

#[derive(Clone, Copy, Debug)]
pub enum Error {
    EchoAlreadyHigh,
    NoEcho,
    EchoStuckHigh,
    OutOfRange,
}

impl Error {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EchoAlreadyHigh => "echo_already_high",
            Self::NoEcho => "no_echo",
            Self::EchoStuckHigh => "echo_stuck_high",
            Self::OutOfRange => "out_of_range",
        }
    }
}

pub const MIN_DISTANCE_MM: u32 = 20;
pub const MAX_DISTANCE_MM: u32 = 4_000;
const ECHO_TIMEOUT_US: u64 = 30_000;

#[derive(Clone, Copy, Debug)]
pub struct Measurement {
    pub echo_us: u32,
    pub distance_mm: u32,
}

pub fn measure(
    trigger: &mut Output<'_>,
    echo: &Input<'_>,
    delay: &Delay,
) -> Result<Measurement, Error> {
    if echo.is_high() {
        return Err(Error::EchoAlreadyHigh);
    }
    trigger.set_low();
    delay.delay_micros(2);
    trigger.set_high();
    delay.delay_micros(10);
    trigger.set_low();

    let waiting = Instant::now();
    while !echo.is_high() {
        if waiting.elapsed().as_micros() >= ECHO_TIMEOUT_US {
            return Err(Error::NoEcho);
        }
    }
    let rising = Instant::now();
    while echo.is_high() {
        if rising.elapsed().as_micros() >= ECHO_TIMEOUT_US {
            return Err(Error::EchoStuckHigh);
        }
    }
    let pulse_us = rising.elapsed().as_micros() as u32;
    let mm = millimeters(pulse_us);
    if !(MIN_DISTANCE_MM..=MAX_DISTANCE_MM).contains(&mm) {
        return Err(Error::OutOfRange);
    }
    Ok(Measurement {
        echo_us: pulse_us,
        distance_mm: mm,
    })
}

pub const fn millimeters(pulse_us: u32) -> u32 {
    // Round-trip flight time at approximately 343 m/s, near room temperature.
    pulse_us * 343 / 2000
}
