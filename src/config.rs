use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{Context, Result};
use ratatui::style::Color;
use serde::{Deserialize, Deserializer};

/// The configuration of the app.
#[derive(Deserialize, Debug, PartialEq)]
pub struct Config {
    #[serde(deserialize_with = "deserialize_path")]
    pub notes_dir: PathBuf,
    #[serde(default)]
    pub ui: UI,
}

/// Deserializes the path with tilde (~) handling.
fn deserialize_path<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    expand_tilde(&raw).map_err(serde::de::Error::custom)
}

/// Expands tilde in a path.
fn expand_tilde(raw: &str) -> Result<PathBuf, String> {
    if raw == "~" {
        return dirs::home_dir()
            .ok_or_else(|| "cannot expand `~`: home directory not found".to_string());
    }

    let rest = raw.strip_prefix("~/").or_else(|| {
        if cfg!(windows) {
            raw.strip_prefix("~\\")
        } else {
            None
        }
    });

    match rest {
        Some(rest) => {
            let home = dirs::home_dir()
                .ok_or_else(|| "cannot expand `~`: home directory not found".to_string())?;
            Ok(home.join(rest))
        }
        None => Ok(PathBuf::from(raw)),
    }
}

impl Config {
    /// Returns a user default implementation of [`Config`].
    ///
    /// # Errors
    /// Will fail if finding the home directory fails.
    pub fn user_default() -> Result<Self> {
        let home_dir = dirs::home_dir()
            .context("could not determine home directory; set `notes_dir` in the config")?;

        Ok(Self {
            notes_dir: Path::new(&home_dir).join("notes"),
            ui: UI::default(),
        })
    }
}

impl FromStr for Config {
    type Err = toml::de::Error;

    /// Parses the config string into a [`Config`] instance.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        toml::from_str::<Self>(s)
    }
}

/// UI-related configuration.
#[derive(Deserialize, Debug, PartialEq)]
#[serde(default)]
pub struct UI {
    pub header: String,
    pub show_guides: bool,
    #[serde(deserialize_with = "deserialize_hex_color")]
    pub accent: Color,
}

/// Deserializes the accent color.
fn deserialize_hex_color<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    let value = value.trim();

    if value.is_empty() {
        return Ok(Color::Blue);
    }

    let hex = value
        .strip_prefix('#')
        .ok_or_else(|| serde::de::Error::custom("color must start with '#'"))?;

    if hex.len() != 6 {
        return Err(serde::de::Error::custom("color must use #RRGGBB format"));
    }

    let color = Color::from_u32(
        u32::from_str_radix(hex, 16)
            .map_err(|_| serde::de::Error::custom("color must be valid #RRGGBB format"))?,
    );

    Ok(color)
}

impl Default for UI {
    fn default() -> Self {
        Self {
            header: String::from(
                "\
▄▄▄▄ ▄▄▄▄ ▄▄▄▄ ▄▄▄▄ ▄▄▄ 
██ █ ██ █  ██  ██ ▀ ██ █
██ █ ██ █  ██  ██▀  ██ █
▀█ █  █ █  ▐█   █ █  █ █
▀▀ ▀ ▀▀▀▀  ▀▀  ▀▀▀▀ ▀▀▀▀",
            ),
            show_guides: true,
            accent: Color::Blue,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color::{Blue, Rgb};

    #[test]
    fn test_parse_config() {
        let file = include_str!("../tests/fixtures/example-config.toml");
        let config = Config::from_str(file).unwrap();

        assert_eq!(Config::user_default().unwrap(), config);
    }

    #[test]
    fn test_expand_tilde() {
        let home = dirs::home_dir().expect("expected to find home path");
        assert_eq!(expand_tilde("~").unwrap(), home);
        assert_eq!(expand_tilde("~/x").unwrap(), home.join("x"));
        assert_eq!(expand_tilde("~/a/b").unwrap(), home.join("a").join("b"));
        assert_eq!(expand_tilde("~x").unwrap(), Path::new("~x"));
        assert_eq!(expand_tilde("/x").unwrap(), Path::new("/x"));
        // windows-only
        if cfg!(windows) {
            assert_eq!(expand_tilde("~\\x").unwrap(), home.join("x"));
        }
    }

    #[test]
    fn test_deserialize_hex_color() {
        let tmp_dir = tempfile::tempdir().expect("Failed to create temp directory");
        let config = format!("notes_dir = \"{}/notes\"\n[ui]\n", tmp_dir.path().display());

        let parse =
            |accent: &str| Config::from_str(&format!("{config}accent = \"{accent}\"")).unwrap();

        assert_eq!(parse("").ui.accent, Blue);
        assert_eq!(parse("#1e90ff").ui.accent, Rgb(0x1e, 0x90, 0xff));
        assert_eq!(parse("#1E90FF").ui.accent, Rgb(0x1e, 0x90, 0xff));
        assert_eq!(parse("  #000000  ").ui.accent, Rgb(0x00, 0x00, 0x00));
        assert_eq!(parse("#ffffff").ui.accent, Rgb(0xff, 0xff, 0xff));

        for invalid in ["1e90ff", "#fff", "#12345g", "#1234567"] {
            let result = Config::from_str(&format!("{config}accent = \"{invalid}\""));
            assert!(result.is_err(), "expected `{invalid}` to be rejected");
        }
    }
}
