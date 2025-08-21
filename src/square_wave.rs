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
