use crate::ascii::AsciiEncoder;
use crate::braille::BrailleEncoder;
use crate::halfblock::HalfBlockEncoder;
use crate::kitty::KittyEncoder;
use crate::protocol::ImageProtocol;
use crate::sixel::SixelEncoder;

#[cfg(feature = "sixel-rs")]
use crate::sixel_rs::SixelRsEncoder;

use std::io::{self, Write};

pub struct RenderResult {
    pub stream: String,
    pub cols: u32,
    pub rows: u32,
}

pub enum Encoder {
    Sixel(Box<SixelEncoder>),
    #[cfg(feature = "sixel-rs")]
    SixelRs(SixelRsEncoder),
    Kitty(KittyEncoder),
    HalfBlock(HalfBlockEncoder),
    Braille(BrailleEncoder),
    Ascii(AsciiEncoder),
}

impl Encoder {
    pub fn new(protocol: ImageProtocol, max_colors: u8) -> crate::error::Result<Self> {
        Self::with_options(protocol, max_colors, 0)
    }

    pub fn with_options(
        protocol: ImageProtocol,
        max_colors: u8,
        background_select: u8,
    ) -> crate::error::Result<Self> {
        match protocol {
            ImageProtocol::Sixel => Ok(Self::Sixel(Box::new(
                SixelEncoder::with_background_select(max_colors, background_select),
            ))),
            ImageProtocol::Kitty => Ok(Self::Kitty(KittyEncoder::new())),
            ImageProtocol::HalfBlock => Ok(Self::HalfBlock(HalfBlockEncoder::new())),
            ImageProtocol::Braille => Ok(Self::Braille(BrailleEncoder::new())),
            ImageProtocol::Ascii => Ok(Self::Ascii(AsciiEncoder::new())),
        }
    }

    #[cfg(feature = "sixel-rs")]
    pub fn new_sixel_rs(
        max_colors: u8,
        diffusion: sixel_rs::optflags::DiffusionMethod,
        quality: sixel_rs::optflags::Quality,
    ) -> crate::error::Result<Self> {
        Ok(Self::SixelRs(SixelRsEncoder::new(
            max_colors, diffusion, quality,
        )?))
    }

    pub fn encode_to_string(&mut self, width: usize, height: usize, data: &[u8]) -> String {
        match self {
            Self::Sixel(e) => e.encode_frame(width, height, data),
            #[cfg(feature = "sixel-rs")]
            Self::SixelRs(_e) => {
                // sixel-rs writes to stdout, can't return string.
                // Fall back to native encoder for this path.
                let mut native = SixelEncoder::new(255);
                native.encode_frame(width, height, data)
            }
            Self::Kitty(e) => {
                let mut buf = Vec::new();
                e.encode_frame(&mut buf, width, height, data, 0, 0)
                    .unwrap_or_default();
                String::from_utf8_lossy(&buf).into_owned()
            }
            Self::HalfBlock(e) => {
                let mut buf = Vec::new();
                e.encode_frame(&mut buf, width, height, data, 0, 0)
                    .unwrap_or_default();
                String::from_utf8_lossy(&buf).into_owned()
            }
            Self::Braille(e) => {
                let mut buf = Vec::new();
                e.encode_frame(&mut buf, width, height, data, 0, 0)
                    .unwrap_or_default();
                String::from_utf8_lossy(&buf).into_owned()
            }
            Self::Ascii(e) => {
                let mut buf = Vec::new();
                e.encode_frame(&mut buf, width, height, data, 0, 0)
                    .unwrap_or_default();
                String::from_utf8_lossy(&buf).into_owned()
            }
        }
    }

    pub fn encode_frame(
        &mut self,
        width: usize,
        height: usize,
        rgb_data: &[u8],
        writer: &mut dyn Write,
    ) -> io::Result<()> {
        match self {
            Self::Sixel(enc) => {
                let data = enc.encode_frame(width, height, rgb_data);
                writer.write_all(data.as_bytes())?;
            }
            #[cfg(feature = "sixel-rs")]
            Self::SixelRs(enc) => {
                enc.encode_frame(width, height, rgb_data)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            }
            Self::Kitty(enc) => {
                enc.encode_frame(writer, width, height, rgb_data, 0, 0)?;
            }
            Self::HalfBlock(enc) => {
                enc.encode_frame(writer, width, height, rgb_data, 0, 0)?;
            }
            Self::Braille(enc) => {
                enc.encode_frame(writer, width, height, rgb_data, 0, 0)?;
            }
            Self::Ascii(enc) => {
                enc.encode_frame(writer, width, height, rgb_data, 0, 0)?;
            }
        }
        writer.flush()
    }

    pub fn encode_frame_at(
        &mut self,
        width: usize,
        height: usize,
        rgb_data: &[u8],
        x: u32,
        y: u32,
        writer: &mut dyn Write,
    ) -> io::Result<()> {
        match self {
            Self::Sixel(enc) => {
                let stream = enc.encode_frame(width, height, rgb_data);
                if x != 0 || y != 0 {
                    write!(writer, "\x1b[{};{}H", y + 1, x + 1)?;
                }
                writer.write_all(stream.as_bytes())?;
            }
            #[cfg(feature = "sixel-rs")]
            Self::SixelRs(enc) => {
                if x != 0 || y != 0 {
                    write!(writer, "\x1b[{};{}H", y + 1, x + 1)?;
                }
                writer.flush()?;
                enc.encode_frame(width, height, rgb_data)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            }
            Self::Kitty(enc) => {
                enc.encode_frame(writer, width, height, rgb_data, x, y)?;
            }
            Self::HalfBlock(enc) => {
                enc.encode_frame(writer, width, height, rgb_data, x, y)?;
            }
            Self::Braille(enc) => {
                enc.encode_frame(writer, width, height, rgb_data, x, y)?;
            }
            Self::Ascii(enc) => {
                enc.encode_frame(writer, width, height, rgb_data, x, y)?;
            }
        }
        writer.flush()
    }

    pub fn render(
        &mut self,
        width: usize,
        height: usize,
        data: &[u8],
        x: u32,
        y: u32,
    ) -> RenderResult {
        let stream = self.encode_to_string(width, height, data);
        let stream = if x != 0 || y != 0 {
            format!("\x1b[{};{}H{}", y + 1, x + 1, stream)
        } else {
            stream
        };
        let (cols, rows) = self.cell_count(width, height);
        RenderResult { stream, cols, rows }
    }

    pub fn protocol_name(&self) -> &'static str {
        match self {
            Self::Sixel(_) => "sixel",
            #[cfg(feature = "sixel-rs")]
            Self::SixelRs(_) => "sixel-rs",
            Self::Kitty(_) => "kitty",
            Self::HalfBlock(_) => "halfblock",
            Self::Braille(_) => "braille",
            Self::Ascii(_) => "ascii",
        }
    }

    pub fn cell_count(&self, pixel_w: usize, pixel_h: usize) -> (u32, u32) {
        let (cw, ch) = crate::terminal::cell_size();
        match self {
            Self::HalfBlock(_) => (pixel_w as u32, (pixel_h as u32).div_ceil(2)),
            Self::Braille(_) => ((pixel_w as u32).div_ceil(2), (pixel_h as u32).div_ceil(4)),
            Self::Ascii(_) => (pixel_w as u32, pixel_h as u32),
            _ => ((pixel_w as u32).div_ceil(cw), (pixel_h as u32).div_ceil(ch)),
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
