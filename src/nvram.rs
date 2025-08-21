//! DS1307 NVRAM Support
//!
//! This module provides an implementation of the [`RtcNvram`] trait for the
//! [`Ds1307`] real-time clock (RTC).

use embedded_hal::i2c::I2c;

pub use rtc_hal::nvram::RtcNvram;

use crate::{Ds1307, error::Error};

/// DS1307 NVRAM starts at register 0x08
const NVRAM_START: u8 = 0x08;

/// DS1307 has 56 bytes of NVRAM (0x08-0x3F)
const NVRAM_SIZE: u8 = 56;

/// 56 NVRAM + 1 address byte
const MAX_NVRAM_WRITE: usize = 57;

impl<I2C, E> Ds1307<I2C>
where
    I2C: I2c<Error = E>,
{
    /// Validate NVRAM offset and length parameters before accessing memory.
    ///
    /// Returns an error if:
    /// - The starting offset is outside the available NVRAM range
    /// - The requested length goes beyond the end of NVRAM
    fn validate_nvram_bounds(&self, offset: u8, len: usize) -> Result<(), Error<E>> {
        // Check if offset is within bounds
        if offset >= NVRAM_SIZE {
            return Err(Error::NvramOutOfBounds);
        }

        // Check if remaining space is sufficient
        let remaining_space = NVRAM_SIZE - offset;
        if len > remaining_space as usize {
            return Err(Error::NvramOutOfBounds);
        }

        Ok(())
    }
}

impl<I2C, E> RtcNvram for Ds1307<I2C>
where
    I2C: I2c<Error = E>,
{
    /// Read data from DS1307 NVRAM.
    ///
    /// - `offset`: starting NVRAM address (0..55)
    /// - `buffer`: output buffer to store the read data
    ///
    /// Performs a sequential read starting at `NVRAM_START + offset`.
    fn read_nvram(&mut self, offset: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        if buffer.is_empty() {
            return Ok(());
        }

        self.validate_nvram_bounds(offset, buffer.len())?;

        let nvram_addr = NVRAM_START + offset;
        self.read_bytes_at_address(nvram_addr, buffer)?;

        Ok(())
    }

    /// Write data into DS1307 NVRAM.
    ///
    /// - `offset`: starting NVRAM address (0..55)
    /// - `data`: slice containing data to write
    ///
    /// Uses either single-byte write or burst write depending on length.
    fn write_nvram(&mut self, offset: u8, data: &[u8]) -> Result<(), Self::Error> {
        if data.is_empty() {
            return Ok(());
        }

        self.validate_nvram_bounds(offset, data.len())?;

        // Burst write
        let mut buffer = [0u8; MAX_NVRAM_WRITE];
        buffer[0] = NVRAM_START + offset;
        buffer[1..data.len() + 1].copy_from_slice(data);

        self.write_raw_bytes(&buffer[..data.len() + 1])?;

        Ok(())
    }

    /// Return the size of DS1307 NVRAM in bytes (56).
    fn nvram_size(&self) -> u16 {
        NVRAM_SIZE as u16
    }
}
