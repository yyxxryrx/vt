use std::env;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageProtocol {
    Sixel,
    Kitty,
    HalfBlock,
    Braille,
    Ascii,
}

pub fn determine_protocol(force: Option<&str>) -> ImageProtocol {
    if let Some(forced) = force {
        match forced.to_lowercase().as_str() {
            "kitty" => return ImageProtocol::Kitty,
            "sixel" => return ImageProtocol::Sixel,
            "halfblock" => return ImageProtocol::HalfBlock,
            "braille" => return ImageProtocol::Braille,
            "ascii" => return ImageProtocol::Ascii,
            _ => {}
        }
    }
    if is_kitty_available() {
        ImageProtocol::Kitty
    } else if is_sixel_available() {
        ImageProtocol::Sixel
    } else {
        ImageProtocol::HalfBlock
    }
}

fn is_kitty_available() -> bool {
    if env::var("KITTY_WINDOW_ID").is_ok() {
        return true;
    }
    if env::var("KITTY_PID").is_ok() {
        return true;
    }
    if env::var("GHOSTTY_RESOURCES_DIR").is_ok() {
        return true;
    }
    if let Ok(tp) = env::var("TERM_PROGRAM") {
        match tp.as_str() {
            "kitty" | "ghostty" | "rio" | "WezTerm" => return true,
            "iterm.app" => {
                if let Ok(v) = env::var("TERM_PROGRAM_VERSION")
                    && version_gte(&v, 3, 4, 0)
                {
                    return true;
                }
            }
            "konsole" => {
                if let Ok(v) = env::var("KONSOLE_VERSION")
                    && v.parse::<u32>().unwrap_or(0) >= 220400
                {
                    return true;
                }
            }
            _ => {}
        }
    }
    if let Ok(term) = env::var("TERM")
        && (term.to_lowercase().contains("kitty") || term == "xterm-ghostty")
    {
        return true;
    }
    false
}

fn is_sixel_available() -> bool {
    if env::var("FOOT_VERSION").is_ok() {
        return true;
    }
    if let Ok(term) = env::var("TERM")
        && term.to_lowercase().starts_with("foot")
    {
        return true;
    }
    if let Ok(tp) = env::var("TERM_PROGRAM") {
        match tp.as_str() {
            "vscode" => {
                if let Ok(v) = env::var("TERM_PROGRAM_VERSION")
                    && version_gte(&v, 1, 80, 0)
                {
                    return true;
                }
            }
            "rio" => {
                if let Ok(v) = env::var("TERM_PROGRAM_VERSION")
                    && version_gte(&v, 12, 0, 0)
                {
                    return true;
                }
            }
            "mintty" => return true,
            "WezTerm" => {
                if let Ok(v) = env::var("WEZTERM_VERSION")
                    && wezterm_sixel_supported(&v)
                {
                    return true;
                }
            }
            "konsole" => {
                if let Ok(v) = env::var("KONSOLE_VERSION")
                    && v.parse::<u32>().unwrap_or(0) >= 220400
                {
                    return true;
                }
            }
            _ => {}
        }
    }
    if let Ok(term) = env::var("TERM")
        && term.to_lowercase().starts_with("mlterm")
    {
        return true;
    }
    false
}

fn version_gte(version_str: &str, major: u32, minor: u32, patch: u32) -> bool {
    let parts: Vec<u32> = version_str
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    let v_major = parts.first().copied().unwrap_or(0);
    let v_minor = parts.get(1).copied().unwrap_or(0);
    let v_patch = parts.get(2).copied().unwrap_or(0);
    (v_major, v_minor, v_patch) >= (major, minor, patch)
}

fn wezterm_sixel_supported(version: &str) -> bool {
    let parts: Vec<u32> = version.split('.').filter_map(|s| s.parse().ok()).collect();
    let year = parts.first().copied().unwrap_or(0);
    let month = parts.get(1).copied().unwrap_or(0);
    (year, month) >= (2022, 6)
}
