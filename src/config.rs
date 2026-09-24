use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use ratatui::style::Color;
use serde::{Deserialize, Deserializer};

/// The configuration of the app.
#[derive(Deserialize, Debug, PartialEq)]
pub struct Config {
    pub notes_dir: PathBuf,
    pub ui: UI,
}

/// UI-related configuration.
#[derive(Deserialize, Debug, PartialEq)]
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

    let color = Color::from_u32(u32::from_str_radix(hex, 16).unwrap());

    Ok(color)
}

impl Default for Config {
    /// Returns a default implementation of [`Config`].
    fn default() -> Self {
        let home_dir = std::env::var("HOME").unwrap();

        Self {
            notes_dir: Path::new(&home_dir).join("notes"),
            ui: UI {
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
}

impl FromStr for Config {
    type Err = toml::de::Error;

    /// Parses the config string into a [`Config`] instance.
    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        toml::from_str::<Self>(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let file = include_str!("../tests/fixtures/example-config.toml");
        let config = Config::from_str(file).unwrap();

        assert_eq!(Config::default(), config);
    }
}
