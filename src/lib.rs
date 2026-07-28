pub mod args;
pub mod ascii;
pub mod audio;
pub mod braille;
pub mod config;
pub mod encoder;
pub mod error;
pub mod halfblock;
pub mod image;
pub mod kitty;
pub mod player;
pub mod protocol;
pub mod sixel;
pub mod terminal;
pub mod video;

#[cfg(feature = "sixel-rs")]
pub mod sixel_rs;

#[cfg(feature = "ratatui")]
pub mod ratatui;

pub use config::Config;
pub use encoder::{Encoder, RenderResult};
pub use error::{Error, Result};
pub use protocol::ImageProtocol;
