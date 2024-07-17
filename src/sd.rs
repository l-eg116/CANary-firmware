use core::{cmp::Ordering, fmt::Write, str::FromStr};

use bxcan::{Frame, StandardId};
use embedded_sdmmc::{self as sdmmc, ShortFileName};
use heapless::{String, Vec};
use rtic_monotonics::Monotonic;
use stm32f1xx_hal::gpio::{Alternate, Pin};

use crate::{app::Mono, spi::*};

// Note : for performances the buffer should be at least 46 bytes since it's the
// number of characters in a standard can log file (including '\n')
const LOG_LINE_LEN: usize = 46;
const READ_BUFFER_SIZE: usize = 64;
const STORE_BUFFER_SIZE: usize = READ_BUFFER_SIZE * 2;

pub type SdCard = sdmmc::SdCard<
    SpiWrapper<(
        Pin<'B', 13, Alternate>,
        Pin<'B', 14>,
        Pin<'B', 15, Alternate>,
    )>,
    OutputPinWrapper<'B', 12>,
    Mono,
>;
pub type VolumeManager = sdmmc::VolumeManager<SdCard, FakeTimeSource, 2, 2>;
pub type Volume<'a> = sdmmc::Volume<'a, SdCard, FakeTimeSource, 2, 2, 1>;
pub type Directory<'a> = sdmmc::Directory<'a, SdCard, FakeTimeSource, 2, 2, 1>;
pub type File<'a> = sdmmc::File<'a, SdCard, FakeTimeSource, 2, 2, 1>;

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

pub fn frame_to_log(frame: &Frame) -> String<LOG_LINE_LEN> {
    let mut log_line = String::<LOG_LINE_LEN>::new();

    let _empty = bxcan::Data::empty();
    let frame_data = frame.data().unwrap_or(&_empty);

    log_line
        .write_fmt(format_args!(
            "({:010}.000000) can0 {:03X}#{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}\n",
            Mono::now().ticks(),
            match frame.id() {
                bxcan::Id::Standard(n) => n.as_raw() as u32,
                bxcan::Id::Extended(n) => n.as_raw(),
            },
            frame_data.get(0).unwrap_or(&0xFF),
            frame_data.get(1).unwrap_or(&0xFF),
            frame_data.get(2).unwrap_or(&0xFF),
            frame_data.get(3).unwrap_or(&0xFF),
            frame_data.get(4).unwrap_or(&0xFF),
            frame_data.get(5).unwrap_or(&0xFF),
            frame_data.get(6).unwrap_or(&0xFF),
            frame_data.get(7).unwrap_or(&0xFF),
        ))
        .expect("frame should fit in line");

    log_line
}

pub fn index_dir<const N: usize>(
    dir: &mut Directory,
    content: &mut Vec<(bool, ShortFileName), N>,
    dirs_only: bool,
) -> Result<(), sdmmc::Error<sdmmc::SdCardError>> {
    dir.iterate_dir(|e| {
        if dirs_only && !e.attributes.is_directory() {
            return;
        }
        let e = (e.attributes.is_directory(), e.name.clone());

        let mut i = 0;
        while i < content.len() {
            if match (content[i].0, e.0) {
                (false, true) => Ordering::Greater,
                (true, false) => Ordering::Less,
                _ => core::str::from_utf8(content[i].1.base_name())
                    .unwrap_or("")
                    .cmp(core::str::from_utf8(e.1.base_name()).unwrap_or("")),
            }
            .is_ge()
            {
                break;
            }
            i += 1;
        }
        let _ = content.insert(i, e);
    })?;

    if content.is_empty() {
        content
            .push((true, ShortFileName::this_dir()))
            .expect("there is space");
    }

    Ok(())
}
