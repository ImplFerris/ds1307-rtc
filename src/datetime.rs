//! # DateTime Module
//!
//! This module provides an implementation of the [`Rtc`] trait for the
//! DS1307 real-time clock (RTC).

use rtc_hal::{bcd, datetime::DateTimeError, rtc::Rtc};

use crate::{Ds1307, registers::Register};

impl<I2C> Rtc for Ds1307<I2C>
where
    I2C: embedded_hal::i2c::I2c,
{
    /// Read the current date and time from the DS1307.
    fn get_datetime(&mut self) -> Result<rtc_hal::datetime::DateTime, Self::Error> {
        // Since DS1307 allows Subsequent registers can be accessed sequentially until a STOP condition is executed
        // Read all 7 registers in one burst operation
        let mut data = [0; 7];
        self.read_register_bytes(Register::Seconds, &mut data)?;

        // Convert from BCD format and extract fields
        let second = bcd::to_decimal(data[0] & 0b0111_1111); // mask CH (clock halt) bit
        let minute = bcd::to_decimal(data[1]);

        // Handle both 12-hour and 24-hour modes for hours
        let raw_hour = data[2];
        let hour = if (raw_hour & 0b0100_0000) != 0 {
            // 12-hour mode
            // Extract the Hour part (4-0 bits)
            let hr = bcd::to_decimal(raw_hour & 0b0001_1111);
            // Extract the AM/PM (5th bit). if it is set, then it is PM
            let pm = (raw_hour & 0b0010_0000) != 0;

            // Convert it to 24 hour format:
            if pm && hr != 12 {
                hr + 12
            } else if !pm && hr == 12 {
                0
            } else {
                hr
            }
        } else {
            // 24-hour mode
            // Extrac the hour value from 5-0 bits
            bcd::to_decimal(raw_hour & 0b0011_1111)
        };

        // let weekday = Weekday::from_number(bcd::to_decimal(data[3]))
        //     .map_err(crate::error::Error::DateTime)?;

        let day_of_month = bcd::to_decimal(data[4]);
        let month = bcd::to_decimal(data[5]);
        let year = 2000 + bcd::to_decimal(data[6]) as u16;

        rtc_hal::datetime::DateTime::new(year, month, day_of_month, hour, minute, second)
            .map_err(crate::error::Error::DateTime)
    }

    /// Set the current date and time in the DS1307.
    fn set_datetime(&mut self, datetime: &rtc_hal::datetime::DateTime) -> Result<(), Self::Error> {
        if datetime.year() < 2000 || datetime.year() > 2099 {
            // DS1307 only allow this date range
            return Err(crate::error::Error::DateTime(DateTimeError::InvalidYear));
        }

        // Prepare data array for burst write (7 registers)
        let mut data = [0u8; 8];
        data[0] = Register::Seconds.addr();

        // Seconds register (0x00)
        // For normal operation, CH bit should be 0 (clock enabled)
        data[1] = bcd::from_decimal(datetime.second()) & 0b0111_1111; // Clear CH bit

        // Minutes register (0x01)
        data[2] = bcd::from_decimal(datetime.minute());

        // Hours register (0x02) - set to 24-hour mode
        // Clear bit 6 (12/24 hour mode bit) to enable 24-hour mode
        data[3] = bcd::from_decimal(datetime.hour()) & 0b0011_1111;

        let weekday = datetime
            .calculate_weekday()
            .map_err(crate::error::Error::DateTime)?;

        // Day of week register (0x03) - 1=Sunday, 7=Saturday
        data[4] = bcd::from_decimal(weekday.to_number());

        // Day of month register (0x04)
        data[5] = bcd::from_decimal(datetime.day_of_month());

        // Month register (0x05)
        data[6] = bcd::from_decimal(datetime.month());

        // Year register (0x06) - only last 2 digits (00-99)
        let year_2digit = (datetime.year() - 2000) as u8;
        data[7] = bcd::from_decimal(year_2digit);

        // Write all 7 registers in one burst operation
        self.write_raw_bytes(&data)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_hal_mock::eh1::i2c::{Mock as I2cMock, Transaction as I2cTrans};
    use rtc_hal::datetime::DateTime;

    fn new_ds1307(i2c: I2cMock) -> Ds1307<I2cMock> {
        Ds1307::new(i2c)
    }

    #[test]
    fn test_get_datetime_24h_mode() {
        // Simulate reading: sec=0x25(25), min=0x59(59), hour=0x23(23h 24h mode),
        // day_of_week=0x04, day_of_month=0x15(15), month=0x08(August), year=0x23(2023)
        let data = [0x25, 0x59, 0x23, 0x04, 0x15, 0x08, 0x23];
        let expectations = [I2cTrans::write_read(
            0x68,
            vec![Register::Seconds.addr()],
            data.to_vec(),
        )];
        let mut ds1307 = new_ds1307(I2cMock::new(&expectations));

        let dt = ds1307.get_datetime().unwrap();
        assert_eq!(dt.second(), 25);
        assert_eq!(dt.minute(), 59);
        assert_eq!(dt.hour(), 23);

        assert_eq!(dt.day_of_month(), 15);
        assert_eq!(dt.month(), 8);

        assert_eq!(dt.year(), 2023);

        ds1307.release_i2c().done();
    }

    #[test]
    fn test_set_datetime_within_base_century() {
        let datetime = DateTime::new(2025, 8, 27, 15, 30, 45).unwrap();
        // base_century = 20, so 2000-2199 valid. 2023 fits.
        let expectations = [I2cTrans::write(
            0x68,
            vec![
                Register::Seconds.addr(),
                0x45, // sec
                0x30, // min
                0x15, // hour (24h)
                0x04, // weekday (2025-08-27 is Wednesday)
                0x27, // day
                0x8,  // month
                0x25, // year (25)
            ],
        )];

        let mut ds1307 = new_ds1307(I2cMock::new(&expectations));

        ds1307.set_datetime(&datetime).unwrap();

        ds1307.release_i2c().done();
    }

    #[test]
    fn test_set_datetime_invalid_year() {
        let datetime = DateTime::new(1980, 1, 1, 0, 0, 0).unwrap();
        let mut ds1307 = new_ds1307(I2cMock::new(&[]));

        let result = ds1307.set_datetime(&datetime);
        assert!(matches!(
            result,
            Err(crate::error::Error::DateTime(DateTimeError::InvalidYear))
        ));

        ds1307.release_i2c().done();
    }

    #[test]
    fn test_get_datetime_12h_mode_am() {
        // 01:15:30 AM, January 1, 2023 (Sunday)
        let data = [
            0x30,        // seconds = 30
            0x15,        // minutes = 15
            0b0100_0001, // hour register: 12h mode, hr=1, AM
            0x01,        // weekday = Sunday
            0x01,        // day of month
            0x01,        // month = January, century=0
            0x23,        // year = 23
        ];
        let expectations = [I2cTrans::write_read(
            0x68,
            vec![Register::Seconds.addr()],
            data.to_vec(),
        )];
        let mut ds1307 = new_ds1307(I2cMock::new(&expectations));

        let dt = ds1307.get_datetime().unwrap();
        assert_eq!((dt.hour(), dt.minute(), dt.second()), (1, 15, 30));

        ds1307.release_i2c().done();
    }

    #[test]
    fn test_get_datetime_12h_mode_pm() {
        // 11:45:50 PM, December 31, 2023 (Sunday)
        let data = [
            0x50,        // seconds = 50
            0x45,        // minutes = 45
            0b0110_1011, // hour register: 12h mode, hr=11, PM
            0x01,        // weekday = Sunday
            0x31,        // day of month
            0x12,        // month = December
            0x23,        // year = 23
        ];
        let expectations = [I2cTrans::write_read(
            0x68,
            vec![Register::Seconds.addr()],
            data.to_vec(),
        )];
        let mut ds1307 = new_ds1307(I2cMock::new(&expectations));

        let dt = ds1307.get_datetime().unwrap();
        assert_eq!(dt.hour(), 23); // 11 PM -> 23h
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day_of_month(), 31);

        ds1307.release_i2c().done();
    }

    #[test]
    fn test_get_datetime_12h_mode_12am() {
        // 12:10:00 AM, Feb 1, 2023 (Wednesday)
        let data = [
            0x00,        // seconds = 0
            0x10,        // minutes = 10
            0b0101_0010, // 12h mode (bit 6=1), hr=12 (0x12), AM (bit5=0)
            0x03,        // weekday = Tuesday
            0x01,        // day of month
            0x02,        // month = Feb
            0x23,        // year = 23
        ];
        let expectations = [I2cTrans::write_read(
            0x68,
            vec![Register::Seconds.addr()],
            data.to_vec(),
        )];
        let mut ds1307 = new_ds1307(I2cMock::new(&expectations));

        let dt = ds1307.get_datetime().unwrap();
        assert_eq!(dt.hour(), 0); // 12 AM should be 0h
        assert_eq!(dt.minute(), 10);

        ds1307.release_i2c().done();
    }

    #[test]
    fn test_get_datetime_12h_mode_12pm() {
        // 12:45:00 PM, Mar 1, 2023 (Wednesday)
        let data = [
            0x00,        // seconds = 0
            0x45,        // minutes = 45
            0b0111_0010, // 12h mode, hr=12, PM bit set (bit5=1)
            0x04,        // weekday = Wednesday
            0x01,        // day of month
            0x03,        // month = Mar
            0x23,        // year = 23
        ];
        let expectations = [I2cTrans::write_read(
            0x68,
            vec![Register::Seconds.addr()],
            data.to_vec(),
        )];
        let mut ds1307 = new_ds1307(I2cMock::new(&expectations));

        let dt = ds1307.get_datetime().unwrap();
        assert_eq!(dt.hour(), 12); // 12 PM should stay 12h
        assert_eq!(dt.minute(), 45);

        ds1307.release_i2c().done();
    }
}
