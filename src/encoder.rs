use crate::ascii::AsciiEncoder;
use crate::braille::BrailleEncoder;
use crate::halfblock::HalfBlockEncoder;
use crate::kitty::KittyEncoder;
use crate::protocol::ImageProtocol;
use crate::sixel::{DiffusionMethod, Quality, SixelEncoder};

use std::io::{self, Write};

pub enum Encoder {
    Sixel(SixelEncoder),
    Kitty(KittyEncoder),
    HalfBlock(HalfBlockEncoder),
    Braille(BrailleEncoder),
    Ascii(AsciiEncoder),
}

impl Encoder {
    pub fn new(
        protocol: ImageProtocol,
        colors: u8,
        diffusion: DiffusionMethod,
        quality: Quality,
    ) -> crate::error::Result<Self> {
        match protocol {
            ImageProtocol::Sixel => Ok(Self::Sixel(SixelEncoder::new(
                colors,
                diffusion,
                quality,
            )?)),
            ImageProtocol::Kitty => Ok(Self::Kitty(KittyEncoder::new())),
            ImageProtocol::HalfBlock => Ok(Self::HalfBlock(HalfBlockEncoder::new())),
            ImageProtocol::Braille => Ok(Self::Braille(BrailleEncoder::new())),
            ImageProtocol::Ascii => Ok(Self::Ascii(AsciiEncoder::new())),
        }
    }

    pub fn encode_frame(
        &mut self,
        width: usize,
        height: usize,
        rgb_data: &[u8],
        x_off: u32,
        y_off: u32,
    ) -> io::Result<()> {
        match self {
            Self::Sixel(enc) => {
                let mut lock = io::stdout().lock();
                write!(lock, "\x1b[{};{}H", y_off + 1, x_off + 1)?;
                lock.flush()?;
                drop(lock);
                enc.encode_frame(width, height, rgb_data)
                    .map_err(|e| io::Error::other(e.to_string()))?;
            }
            Self::Kitty(enc) => {
                let mut lock = io::stdout().lock();
                write!(lock, "\x1b[{};{}H", y_off + 1, x_off + 1)?;
                enc.encode_frame(&mut lock, width, height, rgb_data, x_off, y_off)?;
            }
            Self::HalfBlock(enc) => {
                let mut lock = io::stdout().lock();
                enc.encode_frame(&mut lock, width, height, rgb_data, x_off, y_off)?;
            }
            Self::Braille(enc) => {
                let mut lock = io::stdout().lock();
                enc.encode_frame(&mut lock, width, height, rgb_data, x_off, y_off)?;
            }
            Self::Ascii(enc) => {
                let mut lock = io::stdout().lock();
                enc.encode_frame(&mut lock, width, height, rgb_data, x_off, y_off)?;
            }
        }
        Ok(())
    }

    pub fn protocol_name(&self) -> &'static str {
        match self {
            Self::Sixel(_) => "sixel",
            Self::Kitty(_) => "kitty",
            Self::HalfBlock(_) => "halfblock",
            Self::Braille(_) => "braille",
            Self::Ascii(_) => "ascii",
        }
    }

    pub fn diffusion_name(&self) -> &'static str {
        match self {
            Self::Sixel(e) => e.diffusion_name(),
            _ => "none",
        }
    }

    pub fn status_row(&self, target_height: u32) -> u32 {
        match self {
            Self::HalfBlock(_) => target_height / 2 + 2,
            Self::Braille(_) => target_height / 4 + 2,
            _ => target_height + 2,
        }
    }
}
