//! Power control implementation for the DS1307
//!
//! This module provides power management functionality for the DS1307 RTC chip,
//! implementing the `RtcPowerControl` trait to allow starting and stopping the
//! internal oscillator that drives timekeeping operations.
//!
//! The DS1307 uses a Clock Halt (CH) bit in the seconds register to control
//! oscillator operation. When set, the oscillator stops and timekeeping is
//! paused. When cleared, the oscillator runs and time advances normally.

use embedded_hal::i2c::I2c;
pub use rtc_hal::control::RtcPowerControl;

use crate::{
    Ds1307,
    registers::{CH_BIT, Register},
};

impl<I2C, E> RtcPowerControl for Ds1307<I2C>
where
    I2C: I2c<Error = E>,
{
    /// Start or resume the RTC oscillator so that timekeeping can continue.
    /// This operation is idempotent - calling it when already running has no effect.
    fn start_clock(&mut self) -> Result<(), Self::Error> {
        // Clear Clock Halt (CH) bit in seconds register to start oscillator
        self.clear_register_bits(Register::Seconds, CH_BIT)
    }

    /// Halt the RTC oscillator, pausing timekeeping until restarted.
    /// This operation is idempotent - calling it when already halted has no effect.
    fn halt_clock(&mut self) -> Result<(), Self::Error> {
        // Set Clock Halt (CH) bit in seconds register to stop oscillator
        self.set_register_bits(Register::Seconds, CH_BIT)
    }
}
