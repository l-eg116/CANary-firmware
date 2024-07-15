use core::cmp::max;

use cortex_m::prelude::*;
use embedded_hal::{
    delay::DelayNs,
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
    pub cs: Pin<'B', 12, Output>,
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
        let mut res = Ok(());
        self.cs.set_high();

        for operation in operations {
            if res.is_err() {
                break;
            }
            match operation {
                Read(words) => {
                    for n in 0..words.len() {
                        if let Err(_) = self.spi.write(&[0x00]) {
                            res = Err(embedded_hal::spi::ErrorKind::Other);
                            break;
                        };
                        words[n] = self.spi.read_data_reg();
                    }
                }
                Write(words) => match self.spi.write(words) {
                    Ok(()) => continue,
                    Err(_) => res = Err(embedded_hal::spi::ErrorKind::Other),
                },
                Transfer(read, write) => {
                    for n in 0..max(read.len(), write.len()) {
                        if let Err(_) = self.spi.write(&[*write.get(n).unwrap_or(&0x00)]) {
                            res = Err(embedded_hal::spi::ErrorKind::Other);
                            break;
                        };
                        if n < read.len() {
                            read[n] = self.spi.read_data_reg();
                        }
                    }
                }
                TransferInPlace(words) => {
                    for n in 0..words.len() {
                        if let Err(_) = self.spi.write(&words[n..n + 1]) {
                            res = Err(embedded_hal::spi::ErrorKind::Other);
                            break;
                        };
                        words[n] = self.spi.read_data_reg();
                    }
                }
                DelayNs(ns) => Mono::delay_ns(&mut Mono, *ns),
            }
        }

        self.cs.set_low();
        return res;
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
