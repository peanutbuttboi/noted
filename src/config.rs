use std::path::{Path, PathBuf};

/// The configuration of the app.
#[derive(Debug)]
pub struct Config {
    pub notes_dir: PathBuf,
    pub header: String,
}

impl Default for Config {
    /// Returns a default implementation of [`Config`].
    fn default() -> Self {
        let home_dir = std::env::var("HOME").unwrap();

        Self {
            notes_dir: Path::new(&home_dir).join("notes"),
            header: String::from(
                "
▄▄▄▄ ▄▄▄▄ ▄▄▄▄ ▄▄▄▄ ▄▄▄ 
██ █ ██ █  ██  ██ ▀ ██ █
██ █ ██ █  ██  ██▀  ██ █
▀█ █  █ █  ▐█   █ █  █ █
▀▀ ▀ ▀▀▀▀  ▀▀  ▀▀▀▀ ▀▀▀▀",
            ),
        }
    }
}

impl Config {
    /// Parses the config file into a [`Config`] instance.
    pub fn parse() -> Self {
        todo!()
    }
}
