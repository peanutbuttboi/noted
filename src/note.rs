use std::{
    fs, io,
    path::{Path, PathBuf},
};

use anyhow::Result;
use thiserror::Error;

/// Errors that can occur while building a [`Note`] from a path.
#[derive(Debug, Error)]
pub enum NoteError {
    /// The path does not point to a markdown (`.md`) file.
    #[error("`{}` is not a markdown file", .0.display())]
    NotMarkdown(PathBuf),

    /// The path exists but is not a regular file (e.g. a directory).
    #[error("`{}` is not a file", .0.display())]
    NotAFile(PathBuf),

    /// The path has no file stem, so a title cannot be derived.
    #[error("`{}` has no title", .0.display())]
    MissingTitle(PathBuf),

    /// The file exists but could not be read (permissions, invalid UTF-8, ...).
    #[error("failed to read `{}`", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// The state of a note.
#[derive(Default, Debug)]
pub struct Note {
    pub title: String,
    pub content: String,
    pub path: PathBuf,
}

impl Note {
    /// Try to build a note from `path`.
    ///
    /// # Errors
    /// Returns a [`NoteError`] when `path` is not a markdown file.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, NoteError> {
        let path = path.as_ref();

        let is_markdown = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"));

        if !is_markdown {
            return Err(NoteError::NotMarkdown(path.to_path_buf()));
        }

        if !path.is_file() {
            return Err(NoteError::NotAFile(path.to_path_buf()));
        }

        let title = path
            .file_stem()
            .ok_or_else(|| NoteError::MissingTitle(path.to_path_buf()))?
            .to_string_lossy()
            .into_owned();

        let content = fs::read_to_string(path).map_err(|source| NoteError::Read {
            path: path.to_path_buf(),
            source,
        })?;

        Ok(Self {
            title,
            content,
            path: path.to_path_buf(),
        })
    }

    /// Updates the contents of a note.
    ///
    /// # Errors
    /// It will fail if reading to a string fails.
    pub fn update_content(&mut self) -> Result<()> {
        self.content = fs::read_to_string(&self.path)?;
        Ok(())
    }
}
