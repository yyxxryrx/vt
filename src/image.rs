use crate::args;
use crate::encoder::Encoder;
use crate::error::Result;
use crate::protocol::ImageProtocol;
use crate::terminal::{CursorGuard, clear_screen, hide_cursor, compute_center_offset, fit_dimensions};

pub fn load_image(path: &str) -> Result<(image::DynamicImage, u32, u32)> {
    let img = image::open(path)?;
    let (w, h) = (img.width(), img.height());
    Ok((img, w, h))
}

pub fn resize_image(img: image::DynamicImage, target_width: u32, target_height: u32) -> Vec<u8> {
    img.resize_exact(
        target_width.max(1),
        target_height.max(1),
        image::imageops::FilterType::Triangle,
    )
    .to_rgb8()
    .to_vec()
}

pub fn is_image_extension(path: &str) -> bool {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    matches!(
        ext.as_str(),
        "jpg"
            | "jpeg"
            | "png"
            | "gif"
            | "webp"
            | "bmp"
            | "tiff"
            | "tif"
            | "ico"
            | "avif"
            | "pnm"
            | "ppm"
            | "pgm"
            | "pbm"
            | "hdr"
            | "qoi"
            | "exr"
    )
}

pub fn run(config: &args::Config, protocol: ImageProtocol) -> Result<()> {
    let (img, orig_w, orig_h) = load_image(&config.path)?;
    let (tw, th) = fit_dimensions(orig_w, orig_h, config.scale, config.size, protocol);

    let rgb_data = resize_image(img, tw, th);

    let (cx, cy) = compute_center_offset(tw, th, protocol, config.center);

    let _guard = CursorGuard;
    let stdout = std::io::stdout();
    let mut stdout_lock = stdout.lock();
    clear_screen(&mut stdout_lock)?;
    hide_cursor(&mut stdout_lock)?;

    let mut enc = Encoder::new(protocol, config.colors, config.diffusion, config.quality)?;
    enc.encode_frame(tw as usize, th as usize, &rgb_data, cx, cy)?;

    Ok(())
}
