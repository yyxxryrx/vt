use crate::config::{Config, parse_size};
use clap::{
    Parser,
    builder::{Styles, styling::AnsiColor},
};

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default().bold())
    .usage(AnsiColor::Yellow.on_default().bold())
    .literal(AnsiColor::Cyan.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Debug, Parser)]
#[command(version, about = "A terminal media player", long_about = None, styles = STYLES)]
pub struct Cli {
    /// Video/Image file path (use - or omit for stdin pipe)
    pub path: Option<String>,

    /// Scale factor
    #[arg(short, long, default_value = "1.0")]
    pub scale: f32,

    /// Number of colors 2-256 (Sixel only)
    #[arg(short, long, default_value = "255")]
    pub colors: u8,

    /// Force protocol: sixel, kitty, halfblock, braille, ascii, auto
    #[arg(short, long)]
    pub protocol: Option<String>,

    /// Enable audio playback
    #[arg(short, long)]
    pub audio: bool,

    /// Show status line
    #[arg(short, long)]
    pub verbose: bool,

    /// Output size in characters (e.g., 80x40)
    #[arg(long)]
    pub size: Option<String>,

    /// Center the output on screen
    #[arg(short = 'C', long)]
    pub center: bool,

    /// Diffusion method for sixel-rs (none, atkinson, fs, stucki, burkes, jajuni, auto)
    #[cfg(feature = "sixel-rs")]
    #[arg(long, default_value = "auto")]
    pub diffusion: String,

    /// Quality for sixel-rs (low, high, full, auto)
    #[cfg(feature = "sixel-rs")]
    #[arg(long, default_value = "auto")]
    pub quality: String,
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

impl From<Cli> for Config {
    fn from(cli: Cli) -> Self {
        let colors = if (2..=255).contains(&cli.colors) {
            cli.colors
        } else {
            println!("Colors must be between 2 and 255, using default 255");
            255
        };

        let size = cli.size.as_deref().and_then(parse_size);

        Config {
            path: cli.path.unwrap_or_default(),
            scale: cli.scale,
            colors,
            force_protocol: cli.protocol,
            verbose: cli.verbose,
            audio: cli.audio,
            size,
            center: cli.center,
            #[cfg(feature = "sixel-rs")]
            diffusion: cli.diffusion,
            #[cfg(feature = "sixel-rs")]
            quality: cli.quality,
        }
    }
}
