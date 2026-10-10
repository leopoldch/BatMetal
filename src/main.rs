#![no_std]
#![no_main]

use esp_hal::{
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig},
    i2c::master::{Config as I2cConfig, I2c, SoftwareTimeout},
    time::{Duration, Instant},
};
use esp_println::println;

mod lcd;
mod sonar;
#[cfg(any(feature = "stepper-test", feature = "stepper-scan"))]
mod stepper;
mod uart;
#[cfg(all(feature = "stepper-test", feature = "stepper-scan"))]
compile_error!("Select stepper-test OR stepper-scan");

esp_bootloader_esp_idf::esp_app_desc!();

const MIN_SONAR_CYCLE_MS: u64 = 65;
const LCD_SCAN_INTERVAL_SAMPLES: u32 = 10;
const LCD_TEST_DURATION_MS: u32 = 5_000;

#[esp_hal::main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();
    let mut green = Output::new(peripherals.GPIO6, Level::Low, OutputConfig::default());
    let mut red = Output::new(peripherals.GPIO7, Level::Low, OutputConfig::default());
    let _console = uart::init(peripherals.UART0, peripherals.GPIO16, peripherals.GPIO17)
        .unwrap_or_else(|err| fail(&mut red, &delay, err));
    let mut trigger = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());
    let echo = Input::new(peripherals.GPIO3, InputConfig::default());
    #[cfg(any(feature = "stepper-test", feature = "stepper-scan"))]
    let mut motor = stepper::Stepper::new([
        Output::new(peripherals.GPIO18, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO19, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO20, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO21, Level::Low, OutputConfig::default()),
    ]);
    let mut i2c = I2c::new(
        peripherals.I2C0,
        I2cConfig::default()
            .with_software_timeout(SoftwareTimeout::Transaction(Duration::from_millis(20))),
    )
    .unwrap_or_else(|err| fail(&mut red, &delay, err))
    .with_sda(peripherals.GPIO10)
    .with_scl(peripherals.GPIO11);

    delay.delay_millis(500);
    println!("\nBatMetal sonar mapping prototype");
    println!("[LED] Green GPIO6 for 1 s, then red GPIO7 for 1 s");
    green.set_high();
    delay.delay_millis(1000);
    green.set_low();
    red.set_high();
    delay.delay_millis(1000);
    red.set_low();
    println!("[LCD] PCF8574T at 0x27; SDA GPIO10 / SCL GPIO11");
    println!("[SONAR] TRIG GPIO2 / ECHO GPIO3 through divider");
    println!("[INFO] Sonar samples follow as one JSON object per line.");
    println!("[INFO] Minimum sonar cycle: 65 ms; echo and LCD time limit the actual rate.");
    if cfg!(any(feature = "stepper-test", feature = "stepper-scan")) {
        println!("[MOTOR] GPIO18/19/20/21 to IN1/2/3/4; LCD disabled in this mode");
    }

    #[cfg(feature = "stepper-test")]
    {
        println!("[MOTOR] IN1=18 IN2=19 IN3=20 IN4=21; test in 10 s");
        delay.delay_millis(10_000);
        motor.move_to(64, &delay);
        delay.delay_millis(500);
        motor.move_to(0, &delay);
        motor.release();
        println!("[MOTOR] 64 half-steps forward and back commanded; verify rotation visually");
    }
    #[cfg(feature = "stepper-scan")]
    {
        println!("[SCAN] Starts in 10 s; manual origin, nominal angles without position feedback");
        delay.delay_millis(10_000);
    }
    #[cfg(feature = "stepper-scan")]
    let mut scan_forward = true;
    #[cfg(feature = "stepper-scan")]
    let mut target = 0;
    let mut sample = 0u32;
    let mut screen = None;
    loop {
        if !cfg!(any(feature = "stepper-test", feature = "stepper-scan"))
            && sample % LCD_SCAN_INTERVAL_SAMPLES == 0
            && screen.is_none()
        {
            let candidate = lcd::Lcd::new();
            if i2c.write(candidate.address, &[]).is_ok() {
                match candidate
                    .init(&mut i2c, &delay)
                    .and_then(|()| candidate.test(&mut i2c, &delay))
                {
                    Ok(()) => {
                        println!("[LCD] Display initialized; test shown for 5 s");
                        delay.delay_millis(LCD_TEST_DURATION_MS);
                        screen = Some(candidate);
                    }
                    Err(err) => println!("[LCD] Initialization failed: {err:?}"),
                }
            } else if sample == 0 || sample % 100 == 0 {
                println!("[LCD] No response at 0x27; retrying");
            }
        }
        #[cfg(feature = "stepper-scan")]
        {
            motor.move_to(target, &delay);
            delay.delay_millis(100);
        }
        let measurement_started = Instant::now();
        let measurement = sonar::measure(&mut trigger, &echo, &delay);
        match measurement {
            Ok(_) => red.set_low(),
            Err(_) => red.set_high(),
        }
        #[cfg(feature = "stepper-scan")]
        log_sample(sample, measurement, motor.position, motor.angle_mdeg());
        #[cfg(not(feature = "stepper-scan"))]
        log_sample(sample, measurement);
        if let Some(lcd) = &screen {
            if let Err(err) = lcd.show(
                &mut i2c,
                &delay,
                measurement.ok().map(|reading| reading.distance_mm),
            ) {
                println!("[LCD] Write failed: {err:?}; rediscovery scheduled");
                screen = None;
            } else if sample % LCD_SCAN_INTERVAL_SAMPLES == 0 {
                println!("[LCD] 32 characters sent without I2C errors");
            }
        }
        #[cfg(feature = "stepper-scan")]
        {
            if target == stepper::SCAN_END {
                scan_forward = false;
            } else if target == 0 && !scan_forward {
                // One complete forward/back sweep; release coils and remain stationary.
                motor.release();
                println!("[SCAN] Complete; coils released. Restart for another scan");
                loop {
                    delay.delay_millis(1000);
                }
            }
            target += if scan_forward {
                stepper::SCAN_INCREMENT
            } else {
                -stepper::SCAN_INCREMENT
            };
        }
        green.toggle();
        sample = sample.wrapping_add(1);
        let elapsed_ms = measurement_started.elapsed().as_millis();
        if elapsed_ms < MIN_SONAR_CYCLE_MS {
            delay.delay_millis((MIN_SONAR_CYCLE_MS - elapsed_ms) as u32);
        }
    }
}

#[cfg(feature = "stepper-scan")]
fn log_sample(
    sample: u32,
    measurement: Result<sonar::Measurement, sonar::Error>,
    step: i32,
    angle_mdeg: i32,
) {
    match measurement {
        Ok(reading) => println!(
            "{{\"sample\":{sample},\"echo_us\":{},\"distance_mm\":{},\"step\":{step},\"angle_mdeg\":{angle_mdeg}}}",
            reading.echo_us,
            reading.distance_mm
        ),
        Err(error) => println!(
            "{{\"sample\":{sample},\"echo_us\":null,\"distance_mm\":null,\"error\":\"{}\",\"step\":{step},\"angle_mdeg\":{angle_mdeg}}}",
            error.as_str()
        ),
    }
}

#[cfg(not(feature = "stepper-scan"))]
fn log_sample(sample: u32, measurement: Result<sonar::Measurement, sonar::Error>) {
    match measurement {
        Ok(reading) => println!(
            "{{\"sample\":{sample},\"echo_us\":{},\"distance_mm\":{}}}",
            reading.echo_us, reading.distance_mm
        ),
        Err(error) => println!(
            "{{\"sample\":{sample},\"echo_us\":null,\"distance_mm\":null,\"error\":\"{}\"}}",
            error.as_str()
        ),
    }
}

fn fail(red: &mut Output<'_>, delay: &Delay, reason: impl core::fmt::Debug) -> ! {
    println!("[ERROR] {reason:?}");
    loop {
        red.toggle();
        delay.delay_millis(500);
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("PANIC: {info}");
    loop {
        core::hint::spin_loop();
    }
}
