mod args;
mod ascii;
mod audio;
mod braille;
mod encoder;
mod error;
mod halfblock;
mod image;
mod kitty;
mod player;
mod protocol;
mod sixel;
mod terminal;
mod video;

use crate::error::Result;
use clap::CommandFactory;

fn main() -> Result<()> {
    let cli = args::Cli::parse_args();
    let config: args::Config = match cli.path.clone() {
        Some(_) => cli.into(),
        None => {
            let mut cmd = args::Cli::command();
            cmd.print_help()?;
            return Ok(());
        }
    };
    let protocol = protocol::determine_protocol(config.force_protocol.as_deref());

    if image::is_image_extension(&config.path) {
        if let Err(e) = image::run(&config, protocol) {
            if config.verbose {
                eprintln!("image crate failed, falling back to FFmpeg: {e}");
            }
            video::run(&config, protocol)?;
        }
    } else {
        video::run(&config, protocol)?;
    }

    Ok(())
}
