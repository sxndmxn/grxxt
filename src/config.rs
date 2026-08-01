//! Configuration parsing for grxxt
//!
//! Reads settings from /etc/greetd/grxxt.toml or an explicit development override.

use anyhow::{Context, Result};
use serde::{de, Deserialize, Deserializer};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};

const CONFIG_PATH: &str = "/etc/greetd/grxxt.toml";
const CONFIG_ENV: &str = "GRXXT_CONFIG";
const DEFAULT_SESSION: &str = "/usr/bin/Hyprland";
const MAX_CONFIG_BYTES: u64 = 64 * 1024;
const MAX_SESSION_BYTES: usize = 16 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_session")]
    pub session: String,

    #[serde(default)]
    pub avatar: Option<String>,

    #[serde(default)]
    pub theme: ThemeConfig,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeConfig {
    #[serde(
        default = "default_background",
        deserialize_with = "deserialize_hex_color"
    )]
    pub background: String,

    #[serde(
        default = "default_foreground",
        deserialize_with = "deserialize_hex_color"
    )]
    pub foreground: String,

    #[serde(default = "default_accent", deserialize_with = "deserialize_hex_color")]
    pub accent: String,

    #[serde(default = "default_error", deserialize_with = "deserialize_hex_color")]
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

fn deserialize_hex_color<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    let Some(hex) = value.strip_prefix('#') else {
        return Err(de::Error::custom("expected a color in #RRGGBB format"));
    };

    if hex.len() == 6 && hex.is_ascii() && hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(value)
    } else {
        Err(de::Error::custom("expected a color in #RRGGBB format"))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            session: default_session(),
            avatar: None,
            theme: ThemeConfig::default(),
        }
    }
}

impl Config {
    /// Load the system configuration or an explicitly overridden path.
    pub fn load() -> Result<Self> {
        let (path, missing_uses_defaults) = config_source(env::var_os(CONFIG_ENV))?;
        Self::load_path(&path, missing_uses_defaults)
    }

    fn load_path(path: &Path, missing_uses_defaults: bool) -> Result<Self> {
        let display_path = path.display();
        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(error)
                if missing_uses_defaults
                    && error.kind() == ErrorKind::NotFound
                    && path_is_absent(path) =>
            {
                return Ok(Self::default());
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to read config: {display_path}"));
            }
        };

        if !metadata.is_file() {
            anyhow::bail!("config is not a regular file: {display_path}");
        }
        if metadata.len() > MAX_CONFIG_BYTES {
            anyhow::bail!("config exceeds {MAX_CONFIG_BYTES} bytes: {display_path}");
        }

        let file = fs::File::open(path)
            .with_context(|| format!("failed to read config: {display_path}"))?;
        let content =
            read_config(file).with_context(|| format!("failed to read config: {display_path}"))?;
        Self::parse(&content).with_context(|| format!("invalid config: {display_path}"))
    }

    fn parse(content: &str) -> Result<Self> {
        let config: Self = toml::from_str(content).context("invalid TOML")?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.session.len() > MAX_SESSION_BYTES {
            anyhow::bail!("session command exceeds {MAX_SESSION_BYTES} bytes");
        }

        let command = shell_words::split(&self.session).context("invalid session command")?;
        if command.is_empty() {
            anyhow::bail!("session command cannot be empty");
        }
        if command.iter().any(|argument| argument.contains('\0')) {
            anyhow::bail!("session command cannot contain NUL bytes");
        }
        Ok(())
    }
}

fn path_is_absent(path: &Path) -> bool {
    matches!(
        fs::symlink_metadata(path),
        Err(error) if error.kind() == ErrorKind::NotFound
    )
}

fn config_source(override_path: Option<OsString>) -> Result<(PathBuf, bool)> {
    match override_path {
        Some(path) if path.is_empty() => anyhow::bail!("{CONFIG_ENV} cannot be empty"),
        Some(path) => Ok((PathBuf::from(path), false)),
        None => Ok((PathBuf::from(CONFIG_PATH), true)),
    }
}

fn read_config(reader: impl Read) -> Result<String> {
    let mut content = Vec::new();
    reader
        .take(MAX_CONFIG_BYTES.saturating_add(1))
        .read_to_end(&mut content)
        .context("failed to read config bytes")?;

    if u64::try_from(content.len()).unwrap_or(u64::MAX) > MAX_CONFIG_BYTES {
        anyhow::bail!("config exceeds {MAX_CONFIG_BYTES} bytes");
    }
    String::from_utf8(content).context("config must be valid UTF-8")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "tests can unwrap")]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.session, DEFAULT_SESSION);
        assert_eq!(config.theme.background, "#0b0a13");
    }

    #[test]
    fn shipped_application_config_matches_defaults() {
        let config = Config::parse(include_str!("../grxxt.toml")).unwrap();
        let defaults = Config::default();

        assert_eq!(config.session, defaults.session);
        assert_eq!(config.avatar, defaults.avatar);
        assert_eq!(config.theme.background, defaults.theme.background);
        assert_eq!(config.theme.foreground, defaults.theme.foreground);
        assert_eq!(config.theme.accent, defaults.theme.accent);
        assert_eq!(config.theme.error, defaults.theme.error);
    }

    #[test]
    fn shipped_greetd_config_uses_installed_binary() {
        let config: toml::Value = toml::from_str(include_str!("../greetd-config.toml")).unwrap();

        assert_eq!(config["terminal"]["vt"].as_integer(), Some(1));
        assert_eq!(
            config["default_session"]["command"].as_str(),
            Some("/usr/local/bin/grxxt")
        );
        assert_eq!(config["default_session"]["user"].as_str(), Some("greeter"));
    }

    #[test]
    fn test_parse_config() {
        let toml = r##"
session = "/bin/bash"

[theme]
background = "#000000"
foreground = "#ffffff"
"##;
        let config = Config::parse(toml).unwrap();
        assert_eq!(config.session, "/bin/bash");
        assert_eq!(config.theme.background, "#000000");
        assert_eq!(config.theme.foreground, "#ffffff");
        // Defaults for unspecified
        assert_eq!(config.theme.accent, "#f1c35f");
    }

    #[test]
    fn malformed_config_is_rejected() {
        let error = Config::parse("session = [not valid").unwrap_err();
        assert!(error.to_string().contains("invalid TOML"));
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let error = Config::parse("sessoin = '/bin/sh'").unwrap_err();
        assert!(format!("{error:#}").contains("unknown field"));
    }

    #[test]
    fn invalid_theme_colors_are_rejected() {
        for color in ["red", "#12345", "#aééx"] {
            let input = format!("[theme]\naccent = '{color}'");
            let error = Config::parse(&input).unwrap_err();
            assert!(format!("{error:#}").contains("#RRGGBB"));
        }
    }

    #[test]
    fn invalid_session_commands_are_rejected_during_config_load() {
        for session in ["", "unterminated '"] {
            let input = format!("session = {session:?}");
            let error = Config::parse(&input).unwrap_err();
            assert!(format!("{error:#}").contains("session command"));
        }

        let oversized = "a".repeat(MAX_SESSION_BYTES + 1);
        let input = format!("session = {oversized:?}");
        let error = Config::parse(&input).unwrap_err();
        assert!(format!("{error:#}").contains("session command exceeds"));
    }

    #[test]
    fn config_input_is_size_bounded() {
        let oversized = "x".repeat(usize::try_from(MAX_CONFIG_BYTES).unwrap() + 1);
        let error = read_config(oversized.as_bytes()).unwrap_err();
        assert!(error.to_string().contains("exceeds"));
    }

    #[test]
    fn config_input_must_be_utf8() {
        let error = read_config([0xff].as_slice()).unwrap_err();
        assert!(format!("{error:#}").contains("valid UTF-8"));
    }

    #[test]
    fn missing_system_config_uses_defaults_but_missing_override_errors() {
        let path = Path::new("/file/that/does/not/exist/grxxt.toml");

        let config = Config::load_path(path, true).unwrap();
        assert_eq!(config.session, DEFAULT_SESSION);

        let error = Config::load_path(path, false).unwrap_err();
        assert!(format!("{error:#}").contains("failed to read config"));
    }

    #[test]
    fn dangling_system_config_symlink_is_an_error() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let link = env::temp_dir().join(format!("grxxt-config-{}-{unique}", std::process::id()));
        let target = link.with_extension("missing-target");
        symlink(&target, &link).unwrap();

        let dangling_result = Config::load_path(&link, true);
        fs::write(&target, "session = '/bin/sh'").unwrap();
        let valid_result = Config::load_path(&link, true);
        fs::remove_file(&link).unwrap();
        fs::remove_file(&target).unwrap();

        let error = dangling_result.unwrap_err();
        assert!(format!("{error:#}").contains("failed to read config"));
        assert_eq!(valid_result.unwrap().session, "/bin/sh");
    }

    #[test]
    fn non_regular_config_is_rejected_before_opening() {
        let error = Config::load_path(&env::temp_dir(), true).unwrap_err();
        assert!(error.to_string().contains("not a regular file"));
    }

    #[test]
    fn only_explicit_override_changes_the_system_config_path() {
        assert_eq!(
            config_source(None).unwrap(),
            (PathBuf::from(CONFIG_PATH), true)
        );
        assert_eq!(
            config_source(Some(OsString::from("grxxt.toml"))).unwrap(),
            (PathBuf::from("grxxt.toml"), false)
        );
        assert!(config_source(Some(OsString::new())).is_err());
    }
}
