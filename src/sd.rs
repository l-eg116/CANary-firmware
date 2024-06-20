use core::str::FromStr;

use bxcan::{Frame, StandardId};
use embedded_sdmmc::{self as sdmmc};
use heapless::{String, Vec};
use rtt_target::rprintln;
use stm32f1xx_hal::gpio::{Alternate, Pin};

use crate::app::Mono;
use crate::spi::*;

// Note : the buffer should be at least 45 bytes since it's the number of characters in a standard can log file
const READ_BUFFER_SIZE: usize = 64;
const STORE_BUFFER_SIZE: usize = READ_BUFFER_SIZE * 2;

pub type VolumeManager = sdmmc::VolumeManager<
    sdmmc::SdCard<
        SpiWrapper<(
            Pin<'B', 13, Alternate>,
            Pin<'B', 14>,
            Pin<'B', 15, Alternate>,
        )>,
        OutputPinWrapper<'B', 12>,
        Mono,
    >,
    FakeTimeSource,
    2,
    2,
>;
pub type Directory<'a> = sdmmc::Directory<
    'a,
    sdmmc::SdCard<
        SpiWrapper<(
            Pin<'B', 13, Alternate>,
            Pin<'B', 14>,
            Pin<'B', 15, Alternate>,
        )>,
        OutputPinWrapper<'B', 12>,
        Mono,
    >,
    FakeTimeSource,
    2,
    2,
    1,
>;
pub type File<'a> = sdmmc::File<
    'a,
    sdmmc::SdCard<
        SpiWrapper<(
            Pin<'B', 13, Alternate>,
            Pin<'B', 14>,
            Pin<'B', 15, Alternate>,
        )>,
        OutputPinWrapper<'B', 12>,
        Mono,
    >,
    FakeTimeSource,
    2,
    2,
    1,
>;

pub fn decode_hex(s: &str) -> Result<Vec<u8, 8>, ()> {
    if s.len() % 2 != 0 {
        Err(())
    } else {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
            .collect()
    }
}

pub struct CanLogsInterator<'a> {
    log_file: File<'a>,
    stored: String<STORE_BUFFER_SIZE>,
}

impl CanLogsInterator<'_> {
    pub fn new(log_file: File) -> CanLogsInterator {
        CanLogsInterator {
            log_file,
            stored: String::new(),
        }
    }
}

impl Iterator for CanLogsInterator<'_> {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.log_file.is_eof() {
            if STORE_BUFFER_SIZE - self.stored.len() >= READ_BUFFER_SIZE {
                let mut buffer = [0u8; READ_BUFFER_SIZE];
                let read_count = self.log_file.read(&mut buffer).unwrap();
                self.stored
                    .push_str(core::str::from_utf8(&buffer[..read_count]).unwrap()) // TODO : error "buffer spilt a utf8 char"
                    .expect("it fits");
            }

            let new_line_i = self.stored.find("\n");
            if let None = new_line_i {
                if STORE_BUFFER_SIZE - self.stored.len() >= READ_BUFFER_SIZE {
                    continue;
                } else {
                    break;
                }
            }
            let new_line_i = new_line_i.expect("some");
            let stored_clone = self.stored.clone();

            let log_line = &stored_clone[..new_line_i];
            self.stored = String::from_str(&stored_clone[new_line_i + 1..]).expect("it fits");

            let mut frame_bytes = log_line
                .split(" ")
                .last()
                .unwrap() // TODO : error "empty line"
                .split("#");
            let frame_id = u16::from_str_radix(frame_bytes.next().unwrap(), 16).unwrap(); // TODO : error "expected at least 2 items" & "expected valid hex"
            let frame_data = decode_hex(frame_bytes.next().unwrap()).unwrap(); // TODO : error "expected at least 2 items" & "expected valid hex"

            return Some(Frame::new_data(
                StandardId::new(frame_id).unwrap(), // TODO : error "unvalid frame_id"
                frame_data.into_array::<8>().unwrap(), // TODO : error "frame_data should be exactly 8 bytes"
            ));
        }
        None
    }
}
