use crate::encoder::Encoder;
use crate::protocol::ImageProtocol;
use crate::terminal::cell_size;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, StatefulWidget, Widget},
};
use std::io::Write;
use std::sync::mpsc;
use std::thread;

pub struct FontSize {
    pub width: u32,
    pub height: u32,
}

impl FontSize {
    pub fn from_terminal() -> Self {
        let (w, h) = cell_size();
        Self {
            width: w,
            height: h,
        }
    }

    pub fn cell_to_pixel(&self, cols: u32, rows: u32) -> (u32, u32) {
        (cols * self.width, rows * self.height)
    }

    pub fn pixel_to_cell(&self, px_w: u32, px_h: u32) -> (u32, u32) {
        (
            (px_w + self.width - 1) / self.width,
            (px_h + self.height - 1) / self.height,
        )
    }
}

pub struct ImageWidget {
    encoder_type: ImageProtocol,
    max_colors: u8,
    data: Vec<u8>,
    pixel_width: usize,
    pixel_height: usize,
}

impl ImageWidget {
    pub fn new(
        encoder_type: ImageProtocol,
        max_colors: u8,
        data: Vec<u8>,
        pixel_width: usize,
        pixel_height: usize,
    ) -> Self {
        Self {
            encoder_type,
            max_colors,
            data,
            pixel_width,
            pixel_height,
        }
    }

    pub fn from_image(
        encoder_type: ImageProtocol,
        max_colors: u8,
        img: &image::DynamicImage,
        area: Rect,
    ) -> Self {
        let fs = FontSize::from_terminal();
        let (max_px_w, max_px_h) = fs.cell_to_pixel(area.width as u32, area.height as u32);

        let resized = img.resize_exact(
            max_px_w.max(1) as u32,
            max_px_h.max(1) as u32,
            image::imageops::FilterType::Triangle,
        );
        let rgb = resized.to_rgb8();
        let pw = rgb.width() as usize;
        let ph = rgb.height() as usize;
        let raw = rgb.to_vec();

        Self {
            encoder_type,
            max_colors,
            data: raw,
            pixel_width: pw,
            pixel_height: ph,
        }
    }
}

impl Widget for ImageWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut enc = Encoder::new(self.encoder_type, self.max_colors).unwrap();
        let stream = enc.encode_to_string(self.pixel_width, self.pixel_height, &self.data);

        let stream = if area.x != 0 || area.y != 0 {
            format!("\x1b[{};{}H{}", area.y + 1, area.x + 1, stream)
        } else {
            stream
        };

        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(stream.as_bytes());
        let _ = stdout.flush();

        for row in area.y..area.y + area.height {
            for col in area.x..area.x + area.width {
                buf.set_string(col, row, " ", Style::default());
            }
        }
    }
}

pub struct StatefulImage {
    encoder_type: ImageProtocol,
    max_colors: u8,
}

impl StatefulImage {
    pub fn new(encoder_type: ImageProtocol, max_colors: u8) -> Self {
        Self {
            encoder_type,
            max_colors,
        }
    }
}

impl StatefulWidget for StatefulImage {
    type State = ImageState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if state.source_image.width() == 0 || state.source_image.height() == 0 {
            return;
        }

        let fs = FontSize::from_terminal();
        let (max_px_w, max_px_h) = fs.cell_to_pixel(area.width as u32, area.height as u32);

        let resized = state.source_image.clone().resize_exact(
            max_px_w.max(1) as u32,
            max_px_h.max(1) as u32,
            image::imageops::FilterType::Triangle,
        );
        let rgb = resized.to_rgb8();
        let pw = rgb.width() as usize;
        let ph = rgb.height() as usize;

        let mut enc = Encoder::new(self.encoder_type, self.max_colors).unwrap();
        let stream = enc.encode_to_string(pw, ph, rgb.as_raw());
        let stream = if area.x != 0 || area.y != 0 {
            format!("\x1b[{};{}H{}", area.y + 1, area.x + 1, stream)
        } else {
            stream
        };

        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(stream.as_bytes());
        let _ = stdout.flush();

        for row in area.y..area.y + area.height {
            for col in area.x..area.x + area.width {
                buf.set_string(col, row, " ", Style::default());
            }
        }
    }
}

pub struct ImageState {
    pub source_image: image::DynamicImage,
}

impl ImageState {
    pub fn new(source_image: image::DynamicImage) -> Self {
        Self { source_image }
    }
}

struct EncodeRequest {
    source: image::DynamicImage,
    area: Rect,
    encoder_type: ImageProtocol,
    max_colors: u8,
}

struct EncodeResult {
    stream: String,
    area: Rect,
}

pub struct ImageWorker {
    tx: mpsc::Sender<EncodeRequest>,
    result_rx: mpsc::Receiver<EncodeResult>,
    pending_area: Option<Rect>,
}

impl ImageWorker {
    pub fn new() -> Self {
        let (req_tx, req_rx) = mpsc::channel::<EncodeRequest>();
        let (res_tx, res_rx) = mpsc::channel::<EncodeResult>();

        thread::spawn(move || {
            while let Ok(req) = req_rx.recv() {
                let fs = FontSize::from_terminal();
                let (max_px_w, max_px_h) =
                    fs.cell_to_pixel(req.area.width as u32, req.area.height as u32);

                let resized = req.source.resize_exact(
                    max_px_w.max(1) as u32,
                    max_px_h.max(1) as u32,
                    image::imageops::FilterType::Triangle,
                );
                let rgb = resized.to_rgb8();
                let pw = rgb.width() as usize;
                let ph = rgb.height() as usize;

                let mut enc = Encoder::new(req.encoder_type, req.max_colors).unwrap();
                let stream = enc.encode_to_string(pw, ph, rgb.as_raw());
                let stream = if req.area.x != 0 || req.area.y != 0 {
                    format!("\x1b[{};{}H{}", req.area.y + 1, req.area.x + 1, stream)
                } else {
                    stream
                };

                let _ = res_tx.send(EncodeResult {
                    stream,
                    area: req.area,
                });
            }
        });

        Self {
            tx: req_tx,
            result_rx: res_rx,
            pending_area: None,
        }
    }

    pub fn request(
        &mut self,
        source: image::DynamicImage,
        area: Rect,
        encoder_type: ImageProtocol,
        max_colors: u8,
    ) {
        self.pending_area = Some(area);
        let _ = self.tx.send(EncodeRequest {
            source,
            area,
            encoder_type,
            max_colors,
        });
    }

    pub fn poll(&self) -> Option<(String, Rect)> {
        self.result_rx.try_recv().ok().map(|r| (r.stream, r.area))
    }
}

pub struct AsyncImageState {
    pub source_image: image::DynamicImage,
    pub cached_stream: Option<String>,
    pub cached_area: Option<Rect>,
    pub loading: bool,
}

impl AsyncImageState {
    pub fn new(source_image: image::DynamicImage) -> Self {
        Self {
            source_image,
            cached_stream: None,
            cached_area: None,
            loading: false,
        }
    }
}

pub struct AsyncImageWidget<'a> {
    worker: &'a mut ImageWorker,
    encoder_type: ImageProtocol,
    max_colors: u8,
}

impl<'a> AsyncImageWidget<'a> {
    pub fn new(worker: &'a mut ImageWorker, encoder_type: ImageProtocol, max_colors: u8) -> Self {
        Self {
            worker,
            encoder_type,
            max_colors,
        }
    }
}

impl StatefulWidget for AsyncImageWidget<'_> {
    type State = AsyncImageState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if let Some((stream, _)) = self.worker.poll() {
            state.cached_stream = Some(stream);
            state.cached_area = Some(area);
            state.loading = false;
        }

        if let Some(ref stream) = state.cached_stream {
            let mut stdout = std::io::stdout();
            let _ = stdout.write_all(stream.as_bytes());
            let _ = stdout.flush();

            if let Some(cached) = state.cached_area {
                for row in cached.y..cached.y + cached.height {
                    for col in cached.x..cached.x + cached.width {
                        buf.set_string(col, row, " ", Style::default());
                    }
                }
            }
        } else if state.loading {
            let loading = Paragraph::new(Line::from(Span::styled(
                "Loading...",
                Style::default().fg(Color::DarkGray),
            )));
            ratatui::widgets::Widget::render(loading, area, buf);
        } else if state.source_image.width() > 0 {
            state.loading = true;
            self.worker.request(
                state.source_image.clone(),
                area,
                self.encoder_type,
                self.max_colors,
            );
            let loading = Paragraph::new(Line::from(Span::styled(
                "Loading...",
                Style::default().fg(Color::DarkGray),
            )));
            ratatui::widgets::Widget::render(loading, area, buf);
        }
    }
}
