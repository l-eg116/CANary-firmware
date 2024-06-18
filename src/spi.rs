use core::cmp::max;

use cortex_m::prelude::*;
use embedded_hal::{
    digital::OutputPin,
    spi::{self, Operation::*, SpiDevice},
};
use embedded_sdmmc::{TimeSource, Timestamp};
use rtic_monotonics::{rtic_time::monotonic::TimerQueueBasedInstant, Monotonic};
use stm32f1xx_hal::{
    gpio::{Output, Pin},
    pac::SPI2,
    spi::*,
};

use crate::app::{Mono, TICK_RATE};

pub struct SpiWrapper<PINS> {
    pub spi: Spi<SPI2, Spi2NoRemap, PINS, u8>,
}

impl<PINS> spi::ErrorType for SpiWrapper<PINS> {
    type Error = spi::ErrorKind;
}

impl<PINS> SpiWrapper<PINS> {
    fn flush(&self) {
        while self.spi.is_busy() {}
    }
}

impl<PINS> SpiDevice for SpiWrapper<PINS> {
    fn transaction(
        &mut self,
        operations: &mut [embedded_hal::spi::Operation<'_, u8>],
    ) -> Result<(), Self::Error> {
        for operation in operations {
            match operation {
                Read(words) => {
                    for n in 0..words.len() {
                        if let Err(_) = self.spi.write(&[0x00]) {
                            return Err(embedded_hal::spi::ErrorKind::Other);
                        };
                        words[n] = self.spi.read_data_reg();
                    }
                }
                Write(words) => {
                    return match self.spi.write(words) {
                        Ok(()) => Ok(()),
                        Err(_) => Err(embedded_hal::spi::ErrorKind::Other),
                    }
                }
                Transfer(read, write) => {
                    for n in 0..max(read.len(), write.len()) {
                        if let Err(_) = self.spi.write(&[*write.get(n).unwrap_or(&0x00)]) {
                            return Err(embedded_hal::spi::ErrorKind::Other);
                        };
                        if n < read.len() {
                            read[n] = self.spi.read_data_reg();
                        }
                    }
                }
                TransferInPlace(words) => {
                    for n in 0..words.len() {
                        if let Err(_) = self.spi.write(&words[n..n + 1]) {
                            return Err(embedded_hal::spi::ErrorKind::Other);
                        };
                        words[n] = self.spi.read_data_reg();
                    }
                }
                DelayNs(_) => unimplemented!(),
            }
        }

        Ok(())
    }
}

pub struct OutputPinWrapper<const P: char, const N: u8> {
    pub pin: Pin<P, N, Output>,
}

impl<const P: char, const N: u8> embedded_hal::digital::ErrorType for OutputPinWrapper<P, N> {
    type Error = embedded_hal::digital::ErrorKind;
}

impl<const P: char, const N: u8> OutputPin for OutputPinWrapper<P, N> {
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.pin.set_high();
        Ok(())
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.pin.set_low();
        Ok(())
    }
}

pub struct FakeTimeSource {}

impl TimeSource for FakeTimeSource {
    fn get_timestamp(&self) -> Timestamp {
        let secs_since_boot = Mono::now().ticks() / TICK_RATE;
        Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: (secs_since_boot / 60 / 60 / 24) as u8,
            hours: (secs_since_boot / 60 / 60 % 24) as u8,
            minutes: (secs_since_boot / 60 % 60) as u8,
            seconds: (secs_since_boot % 60) as u8,
        }
    }
}
