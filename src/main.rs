use vt::{args, config, image, protocol, video};

fn is_stdin_piped() -> bool {
    use std::io::IsTerminal;
    !std::io::stdin().is_terminal()
}

fn main() -> vt::Result<()> {
    let cli = args::Cli::parse_args();

    let use_stdin = cli.path.as_deref() == Some("-") || cli.path.is_none() && is_stdin_piped();

    let mut cfg: config::Config = cli.into();
    if use_stdin {
        cfg.path = "-".to_string();
    }

    let protocol = protocol::determine_protocol(cfg.force_protocol.as_deref());

    if cfg.path == "-" {
        video::run(&cfg, protocol)?;
    } else if image::is_image_extension(&cfg.path) {
        if let Err(e) = image::run(&cfg, protocol) {
            if cfg.verbose {
                eprintln!("image crate failed, falling back to FFmpeg: {e}");
            }
            video::run(&cfg, protocol)?;
        }
    } else {
        video::run(&cfg, protocol)?;
    }

    Ok(())
}
