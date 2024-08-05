use core::cmp::max;

use cortex_m::prelude::*;
use embedded_hal::{
    delay::DelayNs,
    digital::OutputPin,
    spi::{self, Operation::*, SpiDevice},
};
use stm32f1xx_hal::{
    gpio::{Output, Pin},
    pac::SPI2,
    spi::*,
};

use crate::app::Mono;

/// A [`stm32f1xx_hal::spi::Spi`] wrapper implementing the [`embedded_hal::spi::SpiDevice`] trait.
pub struct SpiWrapper<PINS> {
    /// The wrapped [`stm32f1xx_hal::spi::Spi`]
    pub spi: Spi<SPI2, Spi2NoRemap, PINS, u8>,
}

impl<PINS> spi::ErrorType for SpiWrapper<PINS> {
    type Error = spi::ErrorKind;
}

impl<PINS> SpiDevice for SpiWrapper<PINS> {
    fn transaction(
        &mut self,
        operations: &mut [embedded_hal::spi::Operation<'_, u8>],
    ) -> Result<(), Self::Error> {
        let mut res = Ok(());

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

        return res;
    }
}

/// A [`stm32f1xx_hal::gpio::Pin`] wrapper implementing the [`embedded_hal::digital::OutputPin`] trait.
pub struct OutputPinWrapper<const P: char, const N: u8> {
    /// The wrapped [`stm32f1xx_hal::gpio::Pin`]
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
