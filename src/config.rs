//! Configuration parsing for grxxt
//!
//! Reads settings from /etc/greetd/grxxt.toml

use serde::Deserialize;
use std::fs;
use std::path::Path;

const CONFIG_PATH: &str = "/etc/greetd/grxxt.toml";
const DEFAULT_SESSION: &str = "/usr/local/bin/start-hyprland.sh";

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_session")]
    pub session: String,

    #[serde(default)]
    pub theme: ThemeConfig,
}

#[derive(Debug, Deserialize)]
pub struct ThemeConfig {
    #[serde(default = "default_background")]
    pub background: String,

    #[serde(default = "default_foreground")]
    pub foreground: String,

    #[serde(default = "default_accent")]
    pub accent: String,

    #[serde(default = "default_error")]
    pub error: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            background: default_background(),
            foreground: default_foreground(),
            accent: default_accent(),
            error: default_error(),
        }
    }
}

fn default_session() -> String {
    DEFAULT_SESSION.to_string()
}

fn default_background() -> String {
    "#0b0a13".to_string()
}

fn default_foreground() -> String {
    "#f6f1e3".to_string()
}

fn default_accent() -> String {
    "#f1c35f".to_string()
}

fn default_error() -> String {
    "#d14b64".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            session: default_session(),
            theme: ThemeConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from the default path, falling back to defaults
    pub fn load() -> Self {
        Self::load_from(CONFIG_PATH)
    }

    /// Load configuration from a specific path
    pub fn load_from<P: AsRef<Path>>(path: P) -> Self {
        fs::read_to_string(path).map_or_else(
            |_| Self::default(),
            |content| toml::from_str(&content).unwrap_or_default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.session, DEFAULT_SESSION);
        assert_eq!(config.theme.background, "#0b0a13");
    }

    #[test]
    fn test_parse_config() {
        let toml = r##"
session = "/bin/bash"

[theme]
background = "#000000"
foreground = "#ffffff"
"##;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.session, "/bin/bash");
        assert_eq!(config.theme.background, "#000000");
        assert_eq!(config.theme.foreground, "#ffffff");
        // Defaults for unspecified
        assert_eq!(config.theme.accent, "#f1c35f");
    }
}
