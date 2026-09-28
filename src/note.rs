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

    /// The path points to a markdown (`.md`) file but doesn't exist.
    #[error("`{}` does not exist", .0.display())]
    NonExistent(PathBuf),

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
#[derive(PartialEq, Default, Debug)]
pub struct Note {
    pub title: String,
    pub content: String,
    pub path: PathBuf,
}

impl Note {
    /// Try to build a note from `path`.
    ///
    /// # Errors
    /// Returns a [`NoteError`] when handling the `path` fails.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, NoteError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(NoteError::NonExistent(path.to_path_buf()));
        }

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
    /// It will fail if `std::fs::read_to_string` fails.
    pub fn update_content(&mut self) -> Result<()> {
        self.content = fs::read_to_string(&self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_note_from_path() {
        let tmp_dir = tempfile::tempdir().unwrap();

        // Valid markdown file
        let path = tmp_dir.path().join("groceries.md");
        fs::write(&path, "# Groceries\n\n- milk\n").unwrap();

        let note = Note::from_path(&path).unwrap();
        assert_eq!(note.title, "groceries");
        assert_eq!(note.content, "# Groceries\n\n- milk\n");
        assert_eq!(note.path, path);

        // Extensions check
        let upper = tmp_dir.path().join("UPPER.MD");
        fs::write(&upper, "hi").unwrap();
        assert_eq!(Note::from_path(&upper).unwrap().title, "UPPER");

        // Non-markdown file
        let txt = tmp_dir.path().join("notes.txt");
        fs::write(&txt, "").unwrap();
        assert!(matches!(
            Note::from_path(&txt),
            Err(NoteError::NotMarkdown(p)) if p == txt
        ));

        // Non-existant file
        let missing = tmp_dir.path().join("notes.md");
        assert!(matches!(
            Note::from_path(&missing),
            Err(NoteError::NonExistent(p)) if p == missing
        ));

        // Directory
        let dir = tmp_dir.path().join("folder.md");
        fs::create_dir(&dir).unwrap();
        assert!(matches!(
            Note::from_path(&dir),
            Err(NoteError::NotAFile(p)) if p == dir
        ));

        // Non-UTF-8
        let invalid = tmp_dir.path().join("invalid.md");
        fs::write(&invalid, [0xff, 0xfe, 0xfd]).unwrap();
        match Note::from_path(&invalid) {
            Err(NoteError::Read { path, source }) => {
                assert_eq!(path, invalid);
                assert_eq!(source.kind(), std::io::ErrorKind::InvalidData);
            }
            other => panic!("expected NoteError::Read, got {other:?}"),
        }
    }

    #[test]
    fn test_note_update_content() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("note.md");
        fs::write(&path, "before").unwrap();

        let mut note = Note::from_path(&path).unwrap();
        assert_eq!(note.content, "before");

        fs::write(&path, "after").unwrap();
        note.update_content().unwrap();
        assert_eq!(note.content, "after");
    }
}
