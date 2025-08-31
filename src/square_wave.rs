//! DS1307 Square Wave Output Support
//!
//! This module provides an implementation of the [`SquareWave`] trait for the
//! [`Ds1307`] real-time clock (RTC).
//!
//! The DS1307 supports four square wave output frequencies: 1 Hz, 4.096 kHz,
//! 8.192 kHz, and 32.768 kHz. Other frequencies defined in
//! [`SquareWaveFreq`] will result in an error.
//!
//! The square wave can be enabled, disabled, and its frequency adjusted by
//! manipulating the control register of the DS1307 over I2C.

use embedded_hal::i2c::I2c;
pub use rtc_hal::square_wave::SquareWave;
pub use rtc_hal::square_wave::SquareWaveFreq;

use crate::Ds1307;
use crate::error::Error;
use crate::registers::Register;
use crate::registers::{OUT_BIT, RS_MASK, SQWE_BIT};

/// Convert a [`SquareWaveFreq`] into the corresponding DS1307 RS bits.
///
/// Returns an error if the frequency is not supported by the DS1307.
fn freq_to_bits<E>(freq: SquareWaveFreq) -> Result<u8, Error<E>> {
    match freq {
        SquareWaveFreq::Hz1 => Ok(0b0000_0000),
        SquareWaveFreq::Hz4096 => Ok(0b0000_0001),
        SquareWaveFreq::Hz8192 => Ok(0b0000_0010),
        SquareWaveFreq::Hz32768 => Ok(0b0000_0011),
        _ => Err(Error::UnsupportedSqwFrequency),
    }
}

impl<I2C, E> SquareWave for Ds1307<I2C>
where
    I2C: I2c<Error = E>,
{
    /// Enable the square wave output with the given frequency.
    ///
    /// The DS1307 supports four square wave output frequencies:
    ///  - 1 Hz ([`SquareWaveFreq::Hz1`])
    ///  - 4.096 kHz ([`SquareWaveFreq::Hz4096`])
    ///  - 8.192 kHz ([`SquareWaveFreq::Hz8192`])
    ///  - 32.768 kHz ([`SquareWaveFreq::Hz32768`])
    ///
    /// Other frequencies defined in [`SquareWaveFreq`] will result in an error.
    fn start_square_wave(&mut self, freq: SquareWaveFreq) -> Result<(), Self::Error> {
        let rs_bits = freq_to_bits(freq)?;
        let current = self.read_register(Register::Control)?;
        let mut new_value = current;

        // Clear frequency bits and set new ones
        new_value &= !RS_MASK;
        new_value |= rs_bits;

        // Enable square wave, disable OUT
        new_value |= SQWE_BIT;
        new_value &= !OUT_BIT;

        // Only write if changed
        if new_value != current {
            self.write_register(Register::Control, new_value)
        } else {
            Ok(())
        }
    }

    /// Enable the square wave output
    fn enable_square_wave(&mut self) -> Result<(), Self::Error> {
        let current = self.read_register(Register::Control)?;
        let mut new_value = current;

        // Enable square wave, disable OUT
        new_value |= SQWE_BIT;
        new_value &= !OUT_BIT;

        // Only write if changed
        if new_value != current {
            self.write_register(Register::Control, new_value)
        } else {
            Ok(())
        }
    }

    /// Disable the square wave output.
    fn disable_square_wave(&mut self) -> Result<(), Self::Error> {
        self.clear_register_bits(Register::Control, SQWE_BIT)
    }

    /// Change the square wave output frequency without enabling or disabling it.
    fn set_square_wave_frequency(&mut self, freq: SquareWaveFreq) -> Result<(), Self::Error> {
        let rs_bits = freq_to_bits(freq)?;
        let current = self.read_register(Register::Control)?;
        let mut new_value = current;

        // Clear frequency bits and set new ones (preserve enable/disable state)
        new_value &= !RS_MASK;
        new_value |= rs_bits;

        // Only write if changed
        if new_value != current {
            self.write_register(Register::Control, new_value)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_hal_mock::eh1::i2c::{Mock as I2cMock, Transaction as I2cTransaction};
    use rtc_hal::square_wave::{SquareWave, SquareWaveFreq};

    const DS1307_ADDR: u8 = 0x68;

    #[test]
    fn test_freq_to_bits_unsupported_frequency() {
        let result = freq_to_bits::<()>(SquareWaveFreq::Hz1024);
        assert!(matches!(result, Err(Error::UnsupportedSqwFrequency)));
    }

    #[test]
    fn test_enable_square_wave() {
        let expectations = vec![
            // Read current control register value
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b1000_0000], // OUT bit set, SQWE bit clear
            ),
            // Write back with SQWE enabled and OUT disabled
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b0001_0000]),
        ];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.enable_square_wave();
        assert!(result.is_ok());

        i2c_mock.done();
    }

    #[test]
    fn test_enable_square_wave_already_enabled() {
        let expectations = vec![I2cTransaction::write_read(
            DS1307_ADDR,
            vec![Register::Control.addr()],
            vec![0b0001_0000], // SQWE already enabled, OUT disabled
        )];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.enable_square_wave();
        assert!(result.is_ok());

        i2c_mock.done();
    }

    #[test]
    fn test_disable_square_wave() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b0001_0011], // SQWE enabled with 32.768kHz
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b0000_0011]),
        ];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.disable_square_wave();
        assert!(result.is_ok());

        i2c_mock.done();
    }

    #[test]
    fn test_set_square_wave_frequency_1hz() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b0001_0011], // SQWE enabled with 32.768kHz
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b0001_0000]),
        ];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.set_square_wave_frequency(SquareWaveFreq::Hz1);
        assert!(result.is_ok());

        i2c_mock.done();
    }

    #[test]
    fn test_set_square_wave_frequency_4096hz() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b0001_0000], // SQWE enabled with 1Hz
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b0001_0001]),
        ];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.set_square_wave_frequency(SquareWaveFreq::Hz4096);
        assert!(result.is_ok());

        i2c_mock.done();
    }

    #[test]
    fn test_set_square_wave_frequency_8192hz() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b0001_0001], // SQWE enabled with 4.096kHz
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b0001_0010]),
        ];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.set_square_wave_frequency(SquareWaveFreq::Hz8192);
        assert!(result.is_ok());

        i2c_mock.done();
    }

    #[test]
    fn test_set_square_wave_frequency_32768hz() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b0001_0000], // SQWE enabled with 1Hz
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b0001_0011]),
        ];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.set_square_wave_frequency(SquareWaveFreq::Hz32768);
        assert!(result.is_ok());

        i2c_mock.done();
    }

    #[test]
    fn test_set_square_wave_frequency_no_change_needed() {
        let expectations = vec![I2cTransaction::write_read(
            DS1307_ADDR,
            vec![Register::Control.addr()],
            vec![0b0001_0010], // Already set to 8.192kHz
        )];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.set_square_wave_frequency(SquareWaveFreq::Hz8192);
        assert!(result.is_ok());

        i2c_mock.done();
    }

    #[test]
    fn test_set_square_wave_frequency_preserves_other_bits() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b1001_0000], // SQWE enabled with other bits set
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b1001_0001]),
        ];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.set_square_wave_frequency(SquareWaveFreq::Hz4096);
        assert!(result.is_ok());

        i2c_mock.done();
    }

    #[test]
    fn test_set_square_wave_frequency_unsupported() {
        let expectations = vec![];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.set_square_wave_frequency(SquareWaveFreq::Hz1024);
        assert!(matches!(result, Err(Error::UnsupportedSqwFrequency)));

        i2c_mock.done();
    }

    #[test]
    fn test_start_square_wave_1hz() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b1000_0011], // OUT enabled with 32.768kHz
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b0001_0000]),
        ];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.start_square_wave(SquareWaveFreq::Hz1);
        assert!(result.is_ok());

        i2c_mock.done();
    }

    #[test]
    fn test_start_square_wave_4096hz() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b0000_0010], // Neither SQWE nor OUT enabled, 8.192kHz bits
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b0001_0001]),
        ];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.start_square_wave(SquareWaveFreq::Hz4096);
        assert!(result.is_ok());

        i2c_mock.done();
    }

    #[test]
    fn test_start_square_wave_8192hz() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b1000_0000], // OUT enabled, no frequency bits
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b0001_0010]),
        ];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.start_square_wave(SquareWaveFreq::Hz8192);
        assert!(result.is_ok());

        i2c_mock.done();
    }

    #[test]
    fn test_start_square_wave_32768hz() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b0000_0001], // 4.096kHz bits, SQWE disabled
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b0001_0011]),
        ];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.start_square_wave(SquareWaveFreq::Hz32768);
        assert!(result.is_ok());

        i2c_mock.done();
    }

    #[test]
    fn test_start_square_wave_already_configured() {
        let expectations = vec![I2cTransaction::write_read(
            DS1307_ADDR,
            vec![Register::Control.addr()],
            vec![0b0001_0001], // Already configured for 4.096kHz
        )];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.start_square_wave(SquareWaveFreq::Hz4096);
        assert!(result.is_ok());

        i2c_mock.done();
    }

    #[test]
    fn test_start_square_wave_preserves_other_bits() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b1100_0010], // Other bits set, 8.192kHz
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b0101_0000]),
        ];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.start_square_wave(SquareWaveFreq::Hz1);
        assert!(result.is_ok());

        i2c_mock.done();
    }

    #[test]
    fn test_start_square_wave_unsupported_frequency() {
        let expectations = vec![];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.start_square_wave(SquareWaveFreq::Hz1024);
        assert!(matches!(result, Err(Error::UnsupportedSqwFrequency)));

        i2c_mock.done();
    }

    #[test]
    fn test_i2c_read_error_handling() {
        let expectations = vec![
            I2cTransaction::write_read(DS1307_ADDR, vec![Register::Control.addr()], vec![0x00])
                .with_error(embedded_hal::i2c::ErrorKind::Other),
        ];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.enable_square_wave();
        assert!(result.is_err());

        i2c_mock.done();
    }

    #[test]
    fn test_i2c_write_error_handling() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b1000_0000],
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b0001_0000])
                .with_error(embedded_hal::i2c::ErrorKind::Other),
        ];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.enable_square_wave();
        assert!(result.is_err());

        i2c_mock.done();
    }

    #[test]
    fn test_rs_mask_coverage() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b1111_1111], // All bits set
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b1111_1100]),
        ];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.set_square_wave_frequency(SquareWaveFreq::Hz1);
        assert!(result.is_ok());

        i2c_mock.done();
    }

    #[test]
    fn test_sqwe_and_out_bit_manipulation() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b1111_1111], // All bits set
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b0111_1111]),
        ];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.enable_square_wave();
        assert!(result.is_ok());

        i2c_mock.done();
    }

    #[test]
    fn test_disable_square_wave_preserves_frequency_bits() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b0001_0011], // SQWE enabled with 32.768kHz
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b0000_0011]),
        ];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.disable_square_wave();
        assert!(result.is_ok());

        i2c_mock.done();
    }

    #[test]
    fn test_enable_square_wave_preserves_frequency_bits() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b1000_0010], // OUT enabled with 8.192kHz
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b0001_0010]),
        ];

        let mut i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(&mut i2c_mock);

        let result = ds1307.enable_square_wave();
        assert!(result.is_ok());

        i2c_mock.done();
    }
}
