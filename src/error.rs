//! Error type definitions for the DS1307 RTC driver.
//!
//! This module defines the `Error` enum and helper functions
//! for classifying and handling DS1307-specific failures.

use rtc_hal::datetime::DateTimeError;

/// DS1307 driver errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error<I2cError> {
    /// I2C communication error
    I2c(I2cError),
    /// Invalid register address
    InvalidAddress,
    /// The specified square wave frequency is not supported by the RTC
    UnsupportedSqwFrequency,
    /// Invalid date/time parameters provided by user
    DateTime(DateTimeError),
    /// NVRAM write would exceed available space
    NvramOutOfBounds,
}

// /// Converts an [`I2cError`] into an [`Error`] by wrapping it in the
// /// [`Error::I2c`] variant.
// ///
impl<I2cError> From<I2cError> for Error<I2cError> {
    fn from(value: I2cError) -> Self {
        Error::I2c(value)
    }
}

impl<I2cError> rtc_hal::error::RtcError for Error<I2cError> {
    fn kind(&self) -> rtc_hal::error::ErrorKind {
        match self {
            Error::I2c(_) => rtc_hal::error::ErrorKind::Bus,
            Error::InvalidAddress => rtc_hal::error::ErrorKind::InvalidAddress,
            Error::DateTime(_) => rtc_hal::error::ErrorKind::InvalidDateTime,
            Error::NvramOutOfBounds => rtc_hal::error::ErrorKind::NvramOutOfBounds,
            Error::UnsupportedSqwFrequency => rtc_hal::error::ErrorKind::UnsupportedSqwFrequency,
        }
    }
}

/// Implements [`defmt::Format`] for [`Error<I2cError>`].
///
/// This enables the error type to be formatted efficiently when logging
/// with the `defmt` framework in `no_std` environments.
///
/// The implementation is only available when the `defmt` feature is enabled
/// and requires that the underlying `I2cError` type also implements
/// [`core::fmt::Debug`].
///
/// Each variant is printed with a short, human-readable description,
/// and the `I2c` variant includes the inner I2C error.
#[cfg(feature = "defmt")]
impl<I2cError> defmt::Format for Error<I2cError>
where
    I2cError: core::fmt::Debug,
{
    fn format(&self, f: defmt::Formatter) {
        match self {
            Error::I2c(e) => {
                defmt::write!(f, "I2C communication error: {:?}", defmt::Debug2Format(e))
            }
            Error::InvalidAddress => defmt::write!(f, "Invalid NVRAM address"),
            Error::DateTime(_) => defmt::write!(f, "Invalid date/time values"),
            Error::NvramOutOfBounds => defmt::write!(f, "NVRAM operation out of bounds"),
            Error::UnsupportedSqwFrequency => defmt::write!(f, "Unsupported Square Wave Frequency"),
        }
    }
}
