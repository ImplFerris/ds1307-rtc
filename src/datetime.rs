//! # DateTime Module
//!
//! This module provides an implementation of the [`Rtc`] trait for the
//! DS1307 real-time clock (RTC).

use embedded_hal::i2c::I2c;
use rtc_hal::{bcd, datetime::DateTimeError, rtc::Rtc};

use crate::{Ds1307, registers::Register};

impl<I2C, E> Rtc for Ds1307<I2C>
where
    I2C: I2c<Error = E>,
{
    type Error = crate::error::Error<E>;

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
