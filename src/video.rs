use crate::args;
use crate::audio;
use crate::error::{Error, Result};
use crate::player;
use crate::protocol::ImageProtocol;
use crate::terminal::{fit_dimensions, is_fzf_preview};
use ffmpeg::{
    format::Pixel,
    software::scaling::{context::Context, flag::Flags},
};
use ffmpeg_next as ffmpeg;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

pub struct VideoDecoder {
    decoder: ffmpeg::codec::decoder::Video,
    scaler: Option<Context>,
    target_width: u32,
    target_height: u32,
    orig_width: u32,
    orig_height: u32,
    decoded_frame: ffmpeg::util::frame::Video,
    rgb_frame: ffmpeg::util::frame::Video,
}

impl VideoDecoder {
    pub fn new(stream: &ffmpeg::format::stream::Stream) -> Result<Self> {
        let context_decoder = ffmpeg::codec::context::Context::from_parameters(stream.parameters())
            .map_err(|e| Error::Ffmpeg(e.to_string()))?;
        let decoder = context_decoder
            .decoder()
            .video()
            .map_err(|e| Error::Ffmpeg(e.to_string()))?;
        let orig_width = decoder.width();
        let orig_height = decoder.height();

        Ok(Self {
            decoder,
            scaler: None,
            target_width: orig_width,
            target_height: orig_height,
            orig_width,
            orig_height,
            decoded_frame: ffmpeg::util::frame::Video::empty(),
            rgb_frame: ffmpeg::util::frame::Video::empty(),
        })
    }

    pub fn original_dimensions(&self) -> (u32, u32) {
        (self.orig_width, self.orig_height)
    }

    pub fn last_frame_pts(&self) -> Option<i64> {
        self.decoded_frame.pts()
    }

    pub fn set_scaling(&mut self, target_width: u32, target_height: u32) -> Result<()> {
        let scaler = Context::get(
            self.decoder.format(),
            self.orig_width,
            self.orig_height,
            Pixel::RGB24,
            target_width,
            target_height,
            Flags::BILINEAR,
        )
        .map_err(|e| Error::Ffmpeg(e.to_string()))?;
        self.scaler = Some(scaler);
        self.target_width = target_width;
        self.target_height = target_height;
        Ok(())
    }

    pub fn process_packet(
        &mut self,
        packet: &ffmpeg::packet::Packet,
        output_buffer: &mut Vec<u8>,
    ) -> Result<bool> {
        let scaler = self.scaler.as_mut().ok_or(Error::ScalingNotSet)?;

        self.decoder
            .send_packet(packet)
            .map_err(|e| Error::Ffmpeg(e.to_string()))?;

        if let Ok(()) = self.decoder.receive_frame(&mut self.decoded_frame) {
            scaler
                .run(&self.decoded_frame, &mut self.rgb_frame)
                .map_err(|e| Error::Ffmpeg(e.to_string()))?;

            let width = self.rgb_frame.width() as usize;
            let height = self.rgb_frame.height() as usize;
            let width_bytes = width * 3;
            let stride = self.rgb_frame.stride(0);
            let total = width_bytes * height;

            output_buffer.resize(total, 0);

            if stride == width_bytes {
                output_buffer.copy_from_slice(&self.rgb_frame.data(0)[..total]);
            } else {
                for y in 0..height {
                    let src = y * stride;
                    let dst = y * width_bytes;
                    output_buffer[dst..dst + width_bytes]
                        .copy_from_slice(&self.rgb_frame.data(0)[src..src + width_bytes]);
                }
            }
            return Ok(true);
        }
        Ok(false)
    }
}

pub fn run(config: &args::Config, protocol: ImageProtocol) -> Result<()> {
    ffmpeg::init()?;
    ffmpeg::log::set_level(ffmpeg::log::Level::Error);

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();
    ctrlc::set_handler(move || {
        running_clone.store(false, Ordering::Release);
    })?;

    let ictx = ffmpeg::format::input(&config.path)?;
    let video_stream = ictx
        .streams()
        .best(ffmpeg::media::Type::Video)
        .ok_or(Error::NoVideoStream)?;
    let video_stream_index = video_stream.index();

    let decoder = VideoDecoder::new(&video_stream)?;
    let (orig_width, orig_height) = decoder.original_dimensions();

    let (target_width, target_height) =
        fit_dimensions(orig_width, orig_height, config.scale, config.size, protocol);

    let audio_stream_info = if config.audio {
        ictx.streams()
            .best(ffmpeg::media::Type::Audio)
            .map(|s| (s.index(), s.parameters()))
    } else {
        None
    };

    let mut audio_player = None;
    let audio_sender = if let Some((audio_stream_index, audio_params)) = audio_stream_info {
        let audio_decoder = ffmpeg::codec::context::Context::from_parameters(audio_params)
            .ok()
            .and_then(|ctx| ctx.decoder().audio().ok());

        if let Some(audio_decoder) = audio_decoder {
            let sample_rate = audio_decoder.rate();
            let channels = audio_decoder.channels();

            if sample_rate > 0 && channels > 0 {
                let (tx, rx) = mpsc::channel();

                let player = audio::AudioPlayer::new(
                    audio::AudioPlayerConfig {
                        sample_rate,
                        channels,
                    },
                    rx,
                    running.clone(),
                );

                if player.is_some() {
                    audio_player = player;
                    Some((
                        tx,
                        audio_stream_index,
                        audio_decoder,
                        channels as i32,
                        sample_rate,
                    ))
                } else {
                    eprintln!("Failed to create audio player, continuing without audio");
                    None
                }
            } else {
                eprintln!("Invalid audio parameters, continuing without audio");
                None
            }
        } else {
            eprintln!("Failed to initialize audio decoder, continuing without audio");
            None
        }
    } else {
        None
    };

    let preview_mode = is_fzf_preview();
    let mut player = player::Player::new(
        &video_stream,
        player::PlayerConfig {
            target_width,
            target_height,
            protocol,
            colors: config.colors,
            diffusion: config.diffusion,
            quality: config.quality,
            verbose: config.verbose,
            preview_mode,
            center: config.center,
        },
    )?;

    if preview_mode && config.verbose {
        eprintln!("fzf preview mode: limiting to first 5 seconds");
    }

    player.run(ictx, video_stream_index, audio_sender, running.clone())?;

    if let Some(mut audio) = audio_player {
        audio.stop();
    }

    Ok(())
}
