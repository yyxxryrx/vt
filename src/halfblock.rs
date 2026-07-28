use std::fmt::Write;
use std::io::{self, Write as IoWrite};

pub struct HalfBlockEncoder;

impl Default for HalfBlockEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl HalfBlockEncoder {
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

        let cap = width * 40 + 16;
        let mut buf = String::with_capacity(cap);

        for (i, y) in (0..height).step_by(2).enumerate() {
            buf.clear();
            let _ = write!(buf, "\x1b[{};{}H", y_off + i as u32 + 1, x_off + 1);
            for x in 0..width {
                let top_offset = (y * width + x) * 3;
                let r_top = rgb_data[top_offset];
                let g_top = rgb_data[top_offset + 1];
                let b_top = rgb_data[top_offset + 2];

                if y + 1 < height {
                    let bot_offset = ((y + 1) * width + x) * 3;
                    let r_bot = rgb_data[bot_offset];
                    let g_bot = rgb_data[bot_offset + 1];
                    let b_bot = rgb_data[bot_offset + 2];
                    let _ = write!(
                        buf,
                        "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m\u{2580}",
                        r_top, g_top, b_top, r_bot, g_bot, b_bot,
                    );
                } else {
                    let _ = write!(buf, "\x1b[38;2;{};{};{}m\u{2580}", r_top, g_top, b_top);
                }
            }
            buf.push_str("\x1b[0m");
            writer.write_all(buf.as_bytes())?;
        }

        Ok(())
    }
}
