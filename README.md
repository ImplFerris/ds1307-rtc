# DS1307 RTC Driver

A Rust driver for the DS1307 Real-Time Clock (RTC) chip, implementing the embedded-hal and rtc-hal traits for integration with embedded Rust projects.

## Features

- Read and set date/time
- Control power (start/stop the clock)
- Use 56 bytes of memory storage
- Generate square wave signals
- Control output pin

## Basic usage

```rust
use ds1307_rtc::Ds1307;
use rtc_hal::rtc::Rtc;  // rtc_hal trait required to be imported to be used
use rtc_hal::datetime::DateTime;

// Set up I2C (depends on your board)
let i2c = /* your I2C setup */;

// Create the driver
let mut rtc = Ds1307::new(i2c);

// Set time to August 21, 2025 at 2:30 PM
let time = DateTime::new(2025, 8, 21, 14, 30, 0).unwrap();
rtc.set_datetime(&time).unwrap();

// Read current time
let now = rtc.get_datetime().unwrap();
```

## License

This project is licensed under the MIT License.
