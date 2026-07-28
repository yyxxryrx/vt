pub fn parse_size(s: &str) -> Option<(u32, u32)> {
    let (w, h) = s.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

pub struct Config {
    pub path: String,
    pub scale: f32,
    pub colors: u8,
    pub force_protocol: Option<String>,
    pub verbose: bool,
    pub audio: bool,
    pub size: Option<(u32, u32)>,
    pub center: bool,
    #[cfg(feature = "sixel-rs")]
    pub diffusion: String,
    #[cfg(feature = "sixel-rs")]
    pub quality: String,
}
