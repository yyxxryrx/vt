use crate::protocol::ImageProtocol;
use std::io::{self, Write};

pub struct CursorGuard;

impl Drop for CursorGuard {
    fn drop(&mut self) {
        let _ = std::io::stdout().write_all(b"\x1b[?25h\n");
        let _ = std::io::stdout().flush();
    }
}

pub fn clear_screen<W: Write>(w: &mut W) -> std::io::Result<()> {
    write!(w, "\x1b[2J")
}

pub fn hide_cursor<W: Write>(w: &mut W) -> io::Result<()> {
    write!(w, "\x1b[?25l")
}

pub fn is_fzf_preview() -> bool {
    std::env::var("FZF_PREVIEW").is_ok() || std::env::var("FZF_PREVIEW_LINES").is_ok()
}

pub fn terminal_size() -> (u32, u32) {
    terminal_size::terminal_size()
        .map(|(w, h)| (w.0 as u32, h.0 as u32))
        .unwrap_or((80, 24))
}

pub fn cell_size() -> (u32, u32) {
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
        let ret = unsafe { libc::ioctl(std::io::stdout().as_raw_fd(), libc::TIOCGWINSZ, &mut ws) };
        if ret == 0 && ws.ws_xpixel > 0 && ws.ws_ypixel > 0 && ws.ws_col > 0 && ws.ws_row > 0 {
            let cw = ws.ws_xpixel as u32 / ws.ws_col as u32;
            let ch = ws.ws_ypixel as u32 / ws.ws_row as u32;
            if cw > 0 && ch > 0 {
                return (cw, ch);
            }
        }
    }
    (9, 18)
}

pub fn fit_dimensions(
    orig_w: u32,
    orig_h: u32,
    scale: f32,
    size: Option<(u32, u32)>,
    protocol: ImageProtocol,
) -> (u32, u32) {
    let (cell_w, cell_h) = cell_size();
    let (px_per_col, px_per_row) = match protocol {
        ImageProtocol::HalfBlock => (1, 2),
        ImageProtocol::Braille => (2, 4),
        ImageProtocol::Ascii => (1, 1),
        _ => (cell_w, cell_h),
    };

    let (phys_bounds_w, phys_bounds_h) = if let Some((cw, ch)) = size {
        (cw * cell_w, ch * cell_h)
    } else {
        let (cols, rows) = terminal_size();
        let max_phys_w = cols * cell_w;
        let max_phys_h = rows.saturating_sub(2) * cell_h;

        if scale < 1.0 {
            (
                (max_phys_w as f64 * scale as f64) as u32,
                (max_phys_h as f64 * scale as f64) as u32,
            )
        } else {
            (max_phys_w, max_phys_h)
        }
    };

    let fit = (phys_bounds_w as f64 / orig_w as f64).min(phys_bounds_h as f64 / orig_h as f64);

    let target_w = (orig_w as f64 * fit * px_per_col as f64 / cell_w as f64) as u32;
    let target_h = (orig_h as f64 * fit * px_per_row as f64 / cell_h as f64) as u32;

    (target_w.max(1), target_h.max(1))
}

pub fn compute_center_offset(
    tw: u32,
    th: u32,
    protocol: ImageProtocol,
    center: bool,
) -> (u32, u32) {
    if !center {
        return (0, 0);
    }

    let (cols, rows) = terminal_size();

    match protocol {
        ImageProtocol::Sixel | ImageProtocol::Kitty => {
            let (cell_w, cell_h) = cell_size();
            let phys_w = cols * cell_w;
            let phys_h = rows * cell_h;
            let cx = ((phys_w.saturating_sub(tw)) / 2) / cell_w;
            let cy = ((phys_h.saturating_sub(th)) / 2) / cell_h;
            (cx, cy)
        }
        ImageProtocol::HalfBlock => {
            let fc = tw;
            let fr = th / 2;
            let cx = if fc < cols { (cols - fc) / 2 } else { 0 };
            let cy = if fr < rows { (rows - fr) / 2 } else { 0 };
            (cx, cy)
        }
        ImageProtocol::Braille => {
            let fc = tw / 2;
            let fr = th / 4;
            let cx = if fc < cols { (cols - fc) / 2 } else { 0 };
            let cy = if fr < rows { (rows - fr) / 2 } else { 0 };
            (cx, cy)
        }
        ImageProtocol::Ascii => {
            let fc = tw;
            let fr = th;
            let cx = if fc < cols { (cols - fc) / 2 } else { 0 };
            let cy = if fr < rows { (rows - fr) / 2 } else { 0 };
            (cx, cy)
        }
    }
}

pub fn clear_image(
    protocol: ImageProtocol,
    pixel_w: u32,
    pixel_h: u32,
    x: u32,
    y: u32,
    writer: &mut dyn Write,
) -> io::Result<()> {
    match protocol {
        ImageProtocol::Kitty => {
            write!(writer, "\x1b_Ga=d,i=1\x1b\\")?;
        }
        ImageProtocol::Sixel => {
            write!(
                writer,
                "\x1b[{};{}H\x1bP0;1;q\"1;1;{};{}\x1b\\",
                y + 1,
                x + 1,
                pixel_w,
                pixel_h
            )?;
        }
        _ => {}
    }
    Ok(())
}
