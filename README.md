# BatMetal

BatMetal is a low-cost sonar mapping prototype built around an ESP32-C6. Its goal
is to turn ultrasonic distance readings into a map of nearby obstacles.

The current prototype measures with an HC-SR04, shows the latest distance on an
LCD1602, and streams JSON records over serial. The trigger interval is at least
65 ms, for a sensor-limited ceiling of about 15 samples per second; echo and
display processing reduce the actual rate. A distance
becomes a map point only when its sensor position and direction are known; this
version does not yet build or display a map automatically.

The HC-SR04's nominal range is **2 cm to 4 m**. The firmware accepts readings in
that interval, times out after 30 ms, and leaves over 60 ms between triggers as
recommended by the sensor datasheet. The advertised range is a best-case
specification: target shape, angle, material, and surroundings affect detection.
[HC-SR04 specifications](https://www.sparkfun.com/ultrasonic-distance-sensor-hc-sr04.html).

## Hardware

- ESP32-C6-DevKitC-1-N8
- HC-SR04 ultrasonic sensor
- LCD1602 with PCF8574T I²C backpack at address `0x27`
- Breadboard, two LEDs, and current-limiting/divider resistors

See the [wiring guide](docs/wiring-guide.md) and [diagram](docs/wiring-esp32-c6.png).
The sonar and LCD use the board's 5 V supply. The sonar ECHO divider protects
the ESP32 input. Keep LCD SDA/SCL pull-ups at or below 3.3 V.

## Build and run

Install the Rust toolchain and `espflash`, then run:

```sh
make check
make flash PORT=/dev/ttyUSB0
```

Use the board's USB-to-UART port to read the 115200-baud console. Valid samples
look like:

```json
{"sample":12,"echo_us":2478,"distance_mm":425}
```

Failed readings include `"distance_mm":null` and an `error` field. Startup and
status messages also use the console; measurement records are the lines beginning
with `{`. Use known sensor angles to place the samples in a 2D map. Manual
positioning is required with the current hardware.
