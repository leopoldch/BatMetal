//! UART0 console routed to the DevKitC-1 USB-to-UART bridge.

use esp_hal::{
    peripherals::{GPIO16, GPIO17, UART0},
    uart::{Config, ConfigError, Uart},
    Blocking,
};

pub fn init<'a>(
    uart: UART0<'a>,
    tx: GPIO16<'a>,
    rx: GPIO17<'a>,
) -> Result<Uart<'a, Blocking>, ConfigError> {
    Ok(Uart::new(uart, Config::default().with_baudrate(115_200))?
        .with_tx(tx)
        .with_rx(rx))
}
