//! Zodiac brutalist theme for ratatui

use ratatui::style::Color;

use crate::config::ThemeConfig;

/// Theme colors for the TUI
#[derive(Debug, Clone)]
pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub accent: Color,
    pub error: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: Color::Rgb(0x0b, 0x0a, 0x13),
            foreground: Color::Rgb(0xf6, 0xf1, 0xe3),
            accent: Color::Rgb(0xf1, 0xc3, 0x5f),
            error: Color::Rgb(0xd1, 0x4b, 0x64),
        }
    }
}

impl From<&ThemeConfig> for Theme {
    fn from(config: &ThemeConfig) -> Self {
        Self {
            background: parse_hex_color(&config.background)
                .unwrap_or(Color::Rgb(0x0b, 0x0a, 0x13)),
            foreground: parse_hex_color(&config.foreground)
                .unwrap_or(Color::Rgb(0xf6, 0xf1, 0xe3)),
            accent: parse_hex_color(&config.accent).unwrap_or(Color::Rgb(0xf1, 0xc3, 0x5f)),
            error: parse_hex_color(&config.error).unwrap_or(Color::Rgb(0xd1, 0x4b, 0x64)),
        }
    }
}

/// Parse a hex color string like "#0b0a13" into a ratatui Color
fn parse_hex_color(s: &str) -> Option<Color> {
    let s = s.strip_prefix('#')?;
    if s.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;

    Some(Color::Rgb(r, g, b))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "tests can unwrap")]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("#0b0a13"), Some(Color::Rgb(0x0b, 0x0a, 0x13)));
        assert_eq!(parse_hex_color("#ffffff"), Some(Color::Rgb(0xff, 0xff, 0xff)));
        assert_eq!(parse_hex_color("#000000"), Some(Color::Rgb(0x00, 0x00, 0x00)));
        assert_eq!(parse_hex_color("invalid"), None);
        assert_eq!(parse_hex_color("#fff"), None);
    }
}
