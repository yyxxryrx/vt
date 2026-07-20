use crate::encoder::Encoder;
use crate::error::Result;
use crate::protocol::ImageProtocol;
use crate::terminal::{CursorGuard, clear_screen, hide_cursor};
use crate::video::VideoDecoder;
use sixel_rs::optflags::{DiffusionMethod, Quality};

use ffmpeg_next as ffmpeg;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

type AudioInfo = (
    mpsc::Sender<Vec<f32>>,
    usize,
    ffmpeg::codec::decoder::Audio,
    i32,
    u32,
);

pub struct PlayerConfig {
    pub target_width: u32,
    pub target_height: u32,
    pub protocol: ImageProtocol,
    pub colors: u8,
    pub diffusion: DiffusionMethod,
    pub quality: Quality,
    pub verbose: bool,
    pub preview_mode: bool,
    pub center: bool,
}

const PREVIEW_MAX_SECS: f64 = 5.0;

pub struct Player {
    decoder: VideoDecoder,
    encoder: Option<Encoder>,
    target_width: u32,
    target_height: u32,
    frame_duration: f64,
    time_base: f64,
    protocol: ImageProtocol,
    colors: u8,
    rgb_buffer: Vec<u8>,
    verbose: bool,
    preview_mode: bool,
    center: bool,
}

impl Player {
    pub fn new(stream: &ffmpeg::Stream, config: PlayerConfig) -> Result<Self> {
        let mut decoder = VideoDecoder::new(stream)?;
        decoder.set_scaling(config.target_width, config.target_height)?;

        let encoder = Some(Encoder::new(
            config.protocol,
            config.colors,
            config.diffusion,
            config.quality,
        )?);

        let fps = {
            let r = stream.avg_frame_rate();
            if r.0 > 0 && r.1 > 0 {
                r.0 as f64 / r.1 as f64
            } else {
                30.0
            }
        };
        let frame_duration = 1.0 / fps;

        let tb = stream.time_base();
        let time_base = tb.0 as f64 / tb.1 as f64;

        let buffer_capacity = (config.target_width * config.target_height * 3) as usize;
        let rgb_buffer = Vec::with_capacity(buffer_capacity);

        Ok(Self {
            decoder,
            encoder,
            target_width: config.target_width,
            target_height: config.target_height,
            frame_duration,
            time_base,
            protocol: config.protocol,
            colors: config.colors,
            rgb_buffer,
            verbose: config.verbose,
            preview_mode: config.preview_mode,
            center: config.center,
        })
    }

    pub fn run(
        &mut self,
        mut ictx: ffmpeg::format::context::Input,
        video_stream_index: usize,
        audio_info: Option<AudioInfo>,
        running: Arc<AtomicBool>,
    ) -> Result<()> {
        let _cursor_guard = CursorGuard;
        {
            let mut lock = std::io::stdout().lock();
            clear_screen(&mut lock)?;
            hide_cursor(&mut lock)?;
        }

        let (center_x, center_y) = crate::terminal::compute_center_offset(
            self.target_width,
            self.target_height,
            self.protocol,
            self.center,
        );

        let start = Instant::now();
        let mut frame_count = 0u32;
        let mut last_frame_time = Instant::now();

        let mut audio_decoder = None;
        let mut audio_resampler = None;
        let mut audio_sender = None;

        if let Some((sender, audio_stream_index, decoder, channels, sample_rate)) = audio_info {
            audio_sender = Some((sender, audio_stream_index));

            let channel_layout = ffmpeg::ChannelLayout::default(channels);
            let resampler = ffmpeg::software::resampling::context::Context::get(
                decoder.format(),
                channel_layout,
                decoder.rate(),
                ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Packed),
                channel_layout,
                sample_rate,
            )
            .ok();

            audio_decoder = Some(decoder);
            audio_resampler = resampler;
        }

        for (stream, packet) in ictx.packets() {
            if !running.load(Ordering::Acquire) {
                break;
            }

            if let Some((ref sender, audio_stream_index)) = audio_sender
                && stream.index() == audio_stream_index
            {
                if let Some(ref mut decoder) = audio_decoder {
                    if decoder.send_packet(&packet).is_err() {
                        continue;
                    }

                    let mut decoded = ffmpeg::util::frame::Audio::empty();
                    while decoder.receive_frame(&mut decoded).is_ok() {
                        if !running.load(Ordering::Acquire) {
                            break;
                        }

                        let samples = if let Some(ref mut resampler) = audio_resampler {
                            let mut resampled = ffmpeg::util::frame::Audio::empty();
                            if resampler.run(&decoded, &mut resampled).is_err() {
                                continue;
                            }
                            let data = resampled.data(0);
                            unsafe {
                                std::slice::from_raw_parts(
                                    data.as_ptr() as *const f32,
                                    data.len() / std::mem::size_of::<f32>(),
                                )
                                .to_vec()
                            }
                        } else {
                            let data = decoded.data(0);
                            unsafe {
                                std::slice::from_raw_parts(
                                    data.as_ptr() as *const f32,
                                    data.len() / std::mem::size_of::<f32>(),
                                )
                                .to_vec()
                            }
                        };

                        if !samples.is_empty() {
                            let _ = sender.send(samples);
                        }
                    }
                }
                continue;
            }

            if stream.index() == video_stream_index
                && self.decoder.process_packet(&packet, &mut self.rgb_buffer)?
            {
                let pts = self.decoder.last_frame_pts();
                if let Some(pts) = pts {
                    let target = start + Duration::from_secs_f64(pts as f64 * self.time_base);
                    let now = Instant::now();
                    if target > now {
                        std::thread::sleep(target - now);
                    }
                } else {
                    let elapsed = last_frame_time.elapsed().as_secs_f64();
                    if elapsed < self.frame_duration {
                        let remaining = self.frame_duration - elapsed;
                        if remaining > 0.001 {
                            std::thread::sleep(Duration::from_secs_f64(remaining));
                        }
                    }
                }
                last_frame_time = Instant::now();

                if let Some(ref mut enc) = self.encoder {
                    enc.encode_frame(
                        self.target_width as usize,
                        self.target_height as usize,
                        &self.rgb_buffer,
                        center_x,
                        center_y,
                    )?;
                }

                frame_count += 1;

                if self.preview_mode
                    && let Some(pts) = self.decoder.last_frame_pts()
                    && pts as f64 * self.time_base >= PREVIEW_MAX_SECS
                {
                    break;
                }

                let elapsed = start.elapsed().as_secs_f64();
                let fps = if elapsed > 0.0 {
                    frame_count as f64 / elapsed
                } else {
                    0.0
                };

                if self.verbose
                    && let Some(ref enc) = self.encoder
                {
                    let protocol_name = enc.protocol_name();
                    let diffusion_name = enc.diffusion_name();
                    let status_row = enc.status_row(self.target_height);
                    let mut lock = std::io::stdout().lock();
                    write!(
                        lock,
                        "\x1b[{};1H\x1b[1mFPS:{:.1} F:{} T:{:.1}s {}x{} C:{} P:{} d:{}\x1b[0m\r",
                        status_row,
                        fps,
                        frame_count,
                        elapsed,
                        self.target_width,
                        self.target_height,
                        self.colors,
                        protocol_name,
                        diffusion_name,
                    )?;
                    lock.flush()?;
                }
            }
        }

        Ok(())
    }
}
