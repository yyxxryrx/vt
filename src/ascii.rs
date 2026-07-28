use std::fmt::Write;
use std::io::{self, Write as IoWrite};

const RAMP: &[u8] = b" .:-=+*#%@";

pub struct AsciiEncoder;

impl Default for AsciiEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl AsciiEncoder {
    pub fn new() -> Self {
        Self
    }

    pub fn encode_frame<W: IoWrite + ?Sized>(
        &mut self,
        writer: &mut W,
        width: usize,
        height: usize,
        rgb_data: &[u8],
        x_off: u32,
        y_off: u32,
    ) -> io::Result<()> {
        if width == 0 || height == 0 || rgb_data.is_empty() {
            return Ok(());
        }

        let cap = width * 28 + 16;
        let mut buf = String::with_capacity(cap);

        for y in 0..height {
            buf.clear();
            let _ = write!(buf, "\x1b[{};{}H", y_off + y as u32 + 1, x_off + 1);
            for x in 0..width {
                let offset = (y * width + x) * 3;
                let r = rgb_data[offset];
                let g = rgb_data[offset + 1];
                let b = rgb_data[offset + 2];

                let luminance = (r as u32 * 77 + g as u32 * 150 + b as u32 * 29) >> 8;
                let idx = luminance * (RAMP.len() - 1) as u32 / 255;
                let ch = RAMP[idx as usize] as char;

                let fg = if luminance > 140 { "30" } else { "37" };
                let _ = write!(buf, "\x1b[48;2;{};{};{}m\x1b[{}m{}", r, g, b, fg, ch);
            }
            buf.push_str("\x1b[0m");
            writer.write_all(buf.as_bytes())?;
        }

        Ok(())
    }
}
