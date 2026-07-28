use crate::error::{Error, Result};
use sixel_rs::encoder::{Encoder, QuickFrameBuilder};
use sixel_rs::optflags::{DiffusionMethod, Quality};
use sixel_rs::pixelformat::PixelFormat;

pub struct SixelRsEncoder {
    encoder: Encoder,
    pixel_buffer: Vec<u8>,
}

impl SixelRsEncoder {
    pub fn new(num_colors: u8, diffusion: DiffusionMethod, quality: Quality) -> Result<Self> {
        let encoder = Encoder::new().map_err(|e| Error::Sixel(format!("{:?}", e)))?;
        encoder
            .set_num_colors(num_colors)
            .map_err(|e| Error::Sixel(format!("set_num_colors: {:?}", e)))?;
        encoder
            .set_diffusion(diffusion)
            .map_err(|e| Error::Sixel(format!("set_diffusion: {:?}", e)))?;
        encoder
            .set_quality(quality)
            .map_err(|e| Error::Sixel(format!("set_quality: {:?}", e)))?;
        Ok(Self {
            encoder,
            pixel_buffer: Vec::new(),
        })
    }

    /// Encode frame directly to stdout (sixel-rs writes to stdout).
    pub fn encode_frame(&mut self, width: usize, height: usize, rgb_data: &[u8]) -> Result<()> {
        let expected_size = width * height * 3;

        self.pixel_buffer.clear();
        if self.pixel_buffer.capacity() < expected_size {
            self.pixel_buffer
                .reserve(expected_size - self.pixel_buffer.capacity());
        }
        self.pixel_buffer.extend_from_slice(rgb_data);

        let frame = QuickFrameBuilder::new()
            .width(width)
            .height(height)
            .format(PixelFormat::RGB888)
            .pixels(self.pixel_buffer.clone());

        self.encoder
            .encode_bytes(frame)
            .map_err(|e| Error::Sixel(format!("encode: {:?}", e)))?;
        Ok(())
    }

    pub fn diffusion_name(&self) -> &'static str {
        "sixel-rs"
    }
}

pub fn diffusion_from_str(s: &str) -> Option<DiffusionMethod> {
    match s.to_lowercase().as_str() {
        "none" => Some(DiffusionMethod::None),
        "atkinson" => Some(DiffusionMethod::Atkinson),
        "fs" | "floyd-steinberg" | "floyd_steinberg" => Some(DiffusionMethod::FS),
        "stucki" => Some(DiffusionMethod::Stucki),
        "burkes" => Some(DiffusionMethod::Burkes),
        "jajuni" => Some(DiffusionMethod::Jajuni),
        "auto" => Some(DiffusionMethod::Auto),
        _ => None,
    }
}

pub fn quality_from_str(s: &str) -> Option<Quality> {
    match s.to_lowercase().as_str() {
        "low" | "l" => Some(Quality::Low),
        "high" | "h" => Some(Quality::High),
        "full" | "f" => Some(Quality::Full),
        "auto" | "a" => Some(Quality::Auto),
        _ => None,
    }
}
