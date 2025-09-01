//! # DS1307 Real-Time Clock Driver

use crate::{
    error::Error,
    registers::{NVRAM_SIZE, OUT_BIT, Register, SQWE_BIT},
};

/// DS1307 I2C device address (fixed)
pub const I2C_ADDR: u8 = 0x68;

/// DS1307 Real-Time Clock driver
pub struct Ds1307<I2C> {
    i2c: I2C,
}

impl<I2C: embedded_hal::i2c::I2c> rtc_hal::error::ErrorType for Ds1307<I2C> {
    type Error = crate::error::Error<I2C::Error>;
}

impl<I2C, E> Ds1307<I2C>
where
    I2C: embedded_hal::i2c::I2c<Error = E>,
    E: core::fmt::Debug,
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

    /// Validate NVRAM offset and length parameters before accessing memory.
    ///
    /// Returns an error if:
    /// - The starting offset is outside the available NVRAM range
    /// - The requested length goes beyond the end of NVRAM
    pub(crate) fn validate_nvram_bounds(&self, offset: u8, len: usize) -> Result<(), Error<E>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registers::{OUT_BIT, Register, SQWE_BIT};
    use embedded_hal_mock::eh1::i2c::{Mock as I2cMock, Transaction as I2cTransaction};

    const DS1307_ADDR: u8 = 0x68;

    #[test]
    fn test_new() {
        let i2c_mock = I2cMock::new(&[]);
        let ds1307 = Ds1307::new(i2c_mock);
        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_release_i2c() {
        let i2c_mock = I2cMock::new(&[]);
        let ds1307 = Ds1307::new(i2c_mock);

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_write_register() {
        let expectations = vec![I2cTransaction::write(
            DS1307_ADDR,
            vec![Register::Control.addr(), 0x42],
        )];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.write_register(Register::Control, 0x42);
        assert!(result.is_ok());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_write_register_error() {
        let expectations = vec![
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0x42])
                .with_error(embedded_hal::i2c::ErrorKind::Other),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.write_register(Register::Control, 0x42);
        assert!(result.is_err());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_read_register() {
        let expectations = vec![I2cTransaction::write_read(
            DS1307_ADDR,
            vec![Register::Control.addr()],
            vec![0x55],
        )];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.read_register(Register::Control);
        assert_eq!(result.unwrap(), 0x55);

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_read_register_error() {
        let expectations = vec![
            I2cTransaction::write_read(DS1307_ADDR, vec![Register::Control.addr()], vec![0x00])
                .with_error(embedded_hal::i2c::ErrorKind::Other),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.read_register(Register::Control);
        assert!(result.is_err());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_read_register_bytes() {
        let expectations = vec![I2cTransaction::write_read(
            DS1307_ADDR,
            vec![Register::Seconds.addr()],
            vec![0x11, 0x22, 0x33],
        )];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let mut buffer = [0u8; 3];
        let result = ds1307.read_register_bytes(Register::Seconds, &mut buffer);
        assert!(result.is_ok());
        assert_eq!(buffer, [0x11, 0x22, 0x33]);

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_read_register_bytes_error() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Seconds.addr()],
                vec![0x00, 0x00],
            )
            .with_error(embedded_hal::i2c::ErrorKind::Other),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let mut buffer = [0u8; 2];
        let result = ds1307.read_register_bytes(Register::Seconds, &mut buffer);
        assert!(result.is_err());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_read_bytes_at_address() {
        let expectations = vec![I2cTransaction::write_read(
            DS1307_ADDR,
            vec![0x08], // Raw address
            vec![0xAA, 0xBB],
        )];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let mut buffer = [0u8; 2];
        let result = ds1307.read_bytes_at_address(0x08, &mut buffer);
        assert!(result.is_ok());
        assert_eq!(buffer, [0xAA, 0xBB]);

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_read_bytes_at_address_error() {
        let expectations = vec![
            I2cTransaction::write_read(DS1307_ADDR, vec![0x08], vec![0x00])
                .with_error(embedded_hal::i2c::ErrorKind::Other),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let mut buffer = [0u8; 1];
        let result = ds1307.read_bytes_at_address(0x08, &mut buffer);
        assert!(result.is_err());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_write_raw_bytes() {
        let expectations = vec![I2cTransaction::write(DS1307_ADDR, vec![0x0E, 0x1C, 0x00])];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.write_raw_bytes(&[0x0E, 0x1C, 0x00]);
        assert!(result.is_ok());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_write_raw_bytes_error() {
        let expectations = vec![
            I2cTransaction::write(DS1307_ADDR, vec![0x0E, 0x1C])
                .with_error(embedded_hal::i2c::ErrorKind::Other),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.write_raw_bytes(&[0x0E, 0x1C]);
        assert!(result.is_err());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_set_register_bits_change_needed() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b0000_1000],
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b0001_1000]),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.set_register_bits(Register::Control, 0b0001_0000);
        assert!(result.is_ok());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_set_register_bits_no_change_needed() {
        let expectations = vec![I2cTransaction::write_read(
            DS1307_ADDR,
            vec![Register::Control.addr()],
            vec![0b0001_1000],
        )];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.set_register_bits(Register::Control, 0b0001_0000);
        assert!(result.is_ok());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_set_register_bits_multiple_bits() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b0000_0000],
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b1010_0101]),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.set_register_bits(Register::Control, 0b1010_0101);
        assert!(result.is_ok());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_set_register_bits_read_error() {
        let expectations = vec![
            I2cTransaction::write_read(DS1307_ADDR, vec![Register::Control.addr()], vec![0x00])
                .with_error(embedded_hal::i2c::ErrorKind::Other),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.set_register_bits(Register::Control, 0b0001_0000);
        assert!(result.is_err());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_set_register_bits_write_error() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b0000_0000],
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b0001_0000])
                .with_error(embedded_hal::i2c::ErrorKind::Other),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.set_register_bits(Register::Control, 0b0001_0000);
        assert!(result.is_err());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_clear_register_bits_change_needed() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b1111_1111],
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b1110_1111]),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.clear_register_bits(Register::Control, 0b0001_0000);
        assert!(result.is_ok());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_clear_register_bits_no_change_needed() {
        let expectations = vec![I2cTransaction::write_read(
            DS1307_ADDR,
            vec![Register::Control.addr()],
            vec![0b1110_1111],
        )];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.clear_register_bits(Register::Control, 0b0001_0000);
        assert!(result.is_ok());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_clear_register_bits_multiple_bits() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b1111_1111],
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b0101_1010]),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.clear_register_bits(Register::Control, 0b1010_0101);
        assert!(result.is_ok());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_clear_register_bits_read_error() {
        let expectations = vec![
            I2cTransaction::write_read(DS1307_ADDR, vec![Register::Control.addr()], vec![0x00])
                .with_error(embedded_hal::i2c::ErrorKind::Other),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.clear_register_bits(Register::Control, 0b0001_0000);
        assert!(result.is_err());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_clear_register_bits_write_error() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b1111_1111],
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b1110_1111])
                .with_error(embedded_hal::i2c::ErrorKind::Other),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.clear_register_bits(Register::Control, 0b0001_0000);
        assert!(result.is_err());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_set_register_bits_preserves_other_bits() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b1000_0010],
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b1001_0010]),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.set_register_bits(Register::Control, 0b0001_0000);
        assert!(result.is_ok());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_clear_register_bits_preserves_other_bits() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b1001_0010],
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b1000_0010]),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.clear_register_bits(Register::Control, 0b0001_0000);
        assert!(result.is_ok());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_set_output_high_from_sqwe_disabled() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b0000_0000], // SQWE=0, OUT=0
            ),
            I2cTransaction::write(
                DS1307_ADDR,
                vec![Register::Control.addr(), OUT_BIT], // SQWE=0, OUT=1
            ),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.set_output_high();
        assert!(result.is_ok());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_set_output_high_from_sqwe_enabled() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![SQWE_BIT], // SQWE=1, OUT=0
            ),
            I2cTransaction::write(
                DS1307_ADDR,
                vec![Register::Control.addr(), OUT_BIT], // SQWE=0, OUT=1
            ),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.set_output_high();
        assert!(result.is_ok());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_set_output_high_already_high() {
        let expectations = vec![I2cTransaction::write_read(
            DS1307_ADDR,
            vec![Register::Control.addr()],
            vec![OUT_BIT], // SQWE=0, OUT=1 (already correct state)
        )];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.set_output_high();
        assert!(result.is_ok());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_set_output_low_from_sqwe_disabled() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![OUT_BIT], // SQWE=0, OUT=1
            ),
            I2cTransaction::write(
                DS1307_ADDR,
                vec![Register::Control.addr(), 0b0000_0000], // SQWE=0, OUT=0
            ),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.set_output_low();
        assert!(result.is_ok());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_set_output_low_from_sqwe_enabled() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![SQWE_BIT | OUT_BIT], // SQWE=1, OUT=1
            ),
            I2cTransaction::write(
                DS1307_ADDR,
                vec![Register::Control.addr(), 0b0000_0000], // SQWE=0, OUT=0
            ),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.set_output_low();
        assert!(result.is_ok());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_set_output_low_already_low() {
        let expectations = vec![I2cTransaction::write_read(
            DS1307_ADDR,
            vec![Register::Control.addr()],
            vec![0b0000_0000], // SQWE=0, OUT=0 (already correct state)
        )];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.set_output_low();
        assert!(result.is_ok());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_set_output_high_read_error() {
        let expectations = vec![
            I2cTransaction::write_read(DS1307_ADDR, vec![Register::Control.addr()], vec![0x00])
                .with_error(embedded_hal::i2c::ErrorKind::Other),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.set_output_high();
        assert!(result.is_err());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_set_output_high_write_error() {
        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![0b0000_0000],
            ),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), OUT_BIT])
                .with_error(embedded_hal::i2c::ErrorKind::Other),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.set_output_high();
        assert!(result.is_err());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_set_output_low_read_error() {
        let expectations = vec![
            I2cTransaction::write_read(DS1307_ADDR, vec![Register::Control.addr()], vec![0x00])
                .with_error(embedded_hal::i2c::ErrorKind::Other),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.set_output_low();
        assert!(result.is_err());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_set_output_low_write_error() {
        let expectations = vec![
            I2cTransaction::write_read(DS1307_ADDR, vec![Register::Control.addr()], vec![OUT_BIT]),
            I2cTransaction::write(DS1307_ADDR, vec![Register::Control.addr(), 0b0000_0000])
                .with_error(embedded_hal::i2c::ErrorKind::Other),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.set_output_low();
        assert!(result.is_err());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }

    #[test]
    fn test_output_functions_preserve_other_bits() {
        // Test that output functions preserve other control register bits
        let other_bits = 0b1100_0000; // Some other bits set

        let expectations = vec![
            I2cTransaction::write_read(
                DS1307_ADDR,
                vec![Register::Control.addr()],
                vec![other_bits | SQWE_BIT], // SQWE enabled with other bits
            ),
            I2cTransaction::write(
                DS1307_ADDR,
                vec![Register::Control.addr(), other_bits | OUT_BIT], // SQWE disabled, OUT high, other bits preserved
            ),
        ];

        let i2c_mock = I2cMock::new(&expectations);
        let mut ds1307 = Ds1307::new(i2c_mock);

        let result = ds1307.set_output_high();
        assert!(result.is_ok());

        let mut i2c_mock = ds1307.release_i2c();
        i2c_mock.done();
    }
}
