//! # DS1307 Real-Time Clock Driver

use embedded_hal::i2c::I2c;

use crate::{
    error::Error,
    registers::{OUT_BIT, Register, SQWE_BIT},
};

/// DS1307 I2C device address (fixed)
pub const I2C_ADDR: u8 = 0x68;

/// DS1307 Real-Time Clock driver
pub struct Ds1307<I2C> {
    i2c: I2C,
}

impl<I2C, E> Ds1307<I2C>
where
    I2C: I2c<Error = E>,
{
    /// Create a new DS1307 driver instance
    ///
    /// # Parameters
    /// * `i2c` - I2C peripheral that implements the embedded-hal I2c trait
    ///
    /// # Returns
    /// New DS1307 driver instance
    pub fn new(i2c: I2C) -> Self {
        Self { i2c }
    }

    /// Returns the underlying I2C bus instance, consuming the driver.
    ///
    /// This allows the user to reuse the I2C bus for other purposes
    /// after the driver is no longer needed.
    ///
    /// However, if you are using [`embedded-hal-bus`](https://crates.io/crates/embedded-hal-bus),
    /// you typically do not need `release_i2c`.
    /// In that case the crate takes care of the sharing
    pub fn release_i2c(self) -> I2C {
        self.i2c
    }

    /// Write a single byte to a DS1307 register
    pub(crate) fn write_register(&mut self, register: Register, value: u8) -> Result<(), Error<E>> {
        self.i2c.write(I2C_ADDR, &[register.addr(), value])?;

        Ok(())
    }

    /// Read a single byte from a DS1307 register
    pub(crate) fn read_register(&mut self, register: Register) -> Result<u8, Error<E>> {
        let mut data = [0u8; 1];
        self.i2c
            .write_read(I2C_ADDR, &[register.addr()], &mut data)
            .map_err(Error::I2c)?;

        Ok(data[0])
    }

    /// Read multiple bytes from DS1307 starting at a register
    pub(crate) fn read_register_bytes(
        &mut self,
        register: Register,
        buffer: &mut [u8],
    ) -> Result<(), Error<E>> {
        self.i2c.write_read(I2C_ADDR, &[register.addr()], buffer)?;

        Ok(())
    }

    /// Read multiple bytes from DS1307 starting at a raw address
    pub(crate) fn read_bytes_at_address(
        &mut self,
        register_addr: u8,
        buffer: &mut [u8],
    ) -> Result<(), Error<E>> {
        self.i2c.write_read(I2C_ADDR, &[register_addr], buffer)?;

        Ok(())
    }

    /// Write raw bytes directly to DS1307 via I2C (register address must be first byte)
    pub(crate) fn write_raw_bytes(&mut self, data: &[u8]) -> Result<(), Error<E>> {
        self.i2c.write(I2C_ADDR, data).map_err(Error::I2c)
    }

    /// Read-modify-write operation for setting bits
    ///
    /// Performs a read-modify-write operation to set the bits specified by the mask
    /// while preserving all other bits in the register. Only performs a write if
    /// the register value would actually change, optimizing I2C bus usage.
    ///
    /// # Parameters
    /// - `register`: The DS1307 register to modify
    /// - `mask`: Bit mask where `1` bits will be set, `0` bits will be ignored
    ///
    /// # Example
    /// ```ignore
    /// // Set bits 2 and 4 in the control register
    /// self.set_register_bits(Register::Control, 0b0001_0100)?;
    /// ```
    ///
    /// # I2C Operations
    /// - 1 read + 1 write (if change needed)
    /// - 1 read only (if no change needed)
    pub(crate) fn set_register_bits(
        &mut self,
        register: Register,
        mask: u8,
    ) -> Result<(), Error<E>> {
        let current = self.read_register(register)?;
        let new_value = current | mask;
        if new_value != current {
            self.write_register(register, new_value)
        } else {
            Ok(())
        }
    }

    /// Read-modify-write operation for clearing bits
    ///
    /// Performs a read-modify-write operation to clear the bits specified by the mask
    /// while preserving all other bits in the register. Only performs a write if
    /// the register value would actually change, optimizing I2C bus usage.
    ///
    /// # Parameters
    /// - `register`: The DS1307 register to modify
    /// - `mask`: Bit mask where `1` bits will be cleared, `0` bits will be ignored
    ///
    /// # Example
    /// ```ignore
    /// // Clear the Clock Halt bit (bit 7) in seconds register
    /// self.clear_register_bits(Register::Seconds, 0b1000_0000)?;
    /// ```
    ///
    /// # I2C Operations
    /// - 1 read + 1 write (if change needed)
    /// - 1 read only (if no change needed)
    pub(crate) fn clear_register_bits(
        &mut self,
        register: Register,
        mask: u8,
    ) -> Result<(), Error<E>> {
        let current = self.read_register(register)?;
        let new_value = current & !mask;
        if new_value != current {
            self.write_register(register, new_value)
        } else {
            Ok(())
        }
    }

    /// Set the output pin to a static high state
    pub fn set_output_high(&mut self) -> Result<(), Error<E>> {
        let current = self.read_register(Register::Control)?;
        let mut new_value = current;

        // Disable square wave and set OUT bit high
        new_value &= !SQWE_BIT;
        new_value |= OUT_BIT;

        if new_value != current {
            self.write_register(Register::Control, new_value)
        } else {
            Ok(())
        }
    }

    /// Set the output pin to a static low state
    pub fn set_output_low(&mut self) -> Result<(), Error<E>> {
        let current = self.read_register(Register::Control)?;
        let mut new_value = current;

        // Disable square wave and set OUT bit low
        new_value &= !SQWE_BIT;
        new_value &= !OUT_BIT;

        if new_value != current {
            self.write_register(Register::Control, new_value)
        } else {
            Ok(())
        }
    }
}
