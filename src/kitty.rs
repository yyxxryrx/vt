use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use std::io::{self, Write};

pub struct KittyEncoder {
    frame_id: u32,
    b64_buffer: Vec<u8>,
}

impl Default for KittyEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl KittyEncoder {
    pub fn new() -> Self {
        Self {
            frame_id: 0,
            b64_buffer: Vec::new(),
        }
    }

    pub fn encode_frame<W: Write + ?Sized>(
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

        self.frame_id += 1;
        let frame_id = self.frame_id;

        write!(writer, "\x1b[{};{}H", y_off + 1, x_off + 1)?;

        let encoded_len = rgb_data.len().div_ceil(3) * 4;
        if self.b64_buffer.len() != encoded_len {
            self.b64_buffer.resize(encoded_len, 0);
        }
        BASE64
            .encode_slice(rgb_data, &mut self.b64_buffer)
            .expect("buffer is exactly sized for base64 output");

        let chunks = self.b64_buffer.chunks(16384);
        let num_chunks = chunks.len();

        for (i, chunk) in chunks.enumerate() {
            let m = if i == num_chunks - 1 { 0 } else { 1 };
            if i == 0 {
                write!(
                    writer,
                    "\x1b_Ga=T,f=24,s={},v={},i={},p=1,q=1,m={};",
                    width, height, frame_id, m
                )?;
            } else {
                write!(writer, "\x1b_Gm={};", m)?;
            }
            writer.write_all(chunk)?;
            write!(writer, "\x1b\\")?;
        }

        Ok(())
    }
}
