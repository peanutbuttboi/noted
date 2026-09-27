use crate::{
    config::Config,
    note::{Note, NoteError},
};

use ratatui::widgets::ListState;
use std::{
    fs::{self, OpenOptions, read_dir},
    path::Path,
};
use unicode_segmentation::UnicodeSegmentation;

use anyhow::{Context, Result, bail};

/// App structure.
#[derive(Debug)]
pub struct App {
    notes: Vec<Note>,
    config: Config,
    list_state: ListState,
    scroll: Scroll,
    screen: Screen,
    status: Option<String>,
}

impl App {
    /// Tries to build an instance of [`App`] with an instance of [`Config`]
    pub fn build(config: Config) -> Result<Self> {
        if !Path::try_exists(&config.notes_dir)? {
            fs::create_dir_all(&config.notes_dir).context("Failed to create note directory")?;
        }

        let dir =
            read_dir(&config.notes_dir).context("Failed to read note directory's contents")?;
        let mut notes = vec![];

        for entry in dir {
            let path = entry?.path();

            match Note::from_path(&path) {
                Ok(note) => notes.push(note),
                Err(NoteError::NotMarkdown(_) | NoteError::NotAFile(_)) => {}
                Err(e) => eprintln!("Warning: Invalid file: {e}"),
            };
        }

        notes.sort_by_cached_key(|a| a.title.to_lowercase());

        let selected = if notes.is_empty() { None } else { Some(0) };

        Ok(Self {
            notes,
            config,
            list_state: ListState::default().with_selected(selected),
            scroll: Scroll::default(),
            screen: Screen::Main,
            status: None,
        })
    }

    /// Returns a reference to `notes`.
    pub fn notes(&self) -> &Vec<Note> {
        &self.notes
    }

    /// Returns a mutable reference to `notes`.
    pub fn note_mut(&mut self, index: usize) -> &mut Note {
        &mut self.notes[index]
    }

    /// Returns a reference to `config`.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns a reference to `list_state`.
    pub fn list_state(&self) -> &ListState {
        &self.list_state
    }

    /// Returns a reference to `screen`.
    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    /// Returns a reference to `status`.
    pub fn status(&self) -> &Option<String> {
        &self.status
    }

    /// Index of the selected note, or None if there is no valid selection.
    pub fn selected_index(&self) -> Option<usize> {
        self.list_state.selected().filter(|&i| i < self.notes.len())
    }

    /// Returns scroll `offset`.
    pub fn scroll_offset(&self) -> u16 {
        self.scroll.offset()
    }

    /// scrolls down `amount`.
    pub fn scroll_down(&mut self, amount: u16) {
        self.scroll.down(amount);
    }

    /// scrolls up `amount`.
    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll.up(amount);
    }

    /// Reconcile with the measurements from the last layout pass.
    pub fn sync_scroll(&mut self, viewport: u16, content: u16) {
        self.scroll.sync(viewport, content);
    }

    /// Sets the screen for creating a new note.
    pub fn begin_new_note(&mut self) {
        self.screen = Screen::NewNote {
            input: String::new(),
        };
    }

    /// Sets the screen for renaming a note.
    ///
    /// # Errors
    /// Will fail if no note is selected.
    pub fn begin_rename_note(&mut self) -> Result<()> {
        let index = self.selected_index().context("No note selected")?;
        let input = self.notes[index].title.clone();
        self.screen = Screen::RenameNote { index, input };
        Ok(())
    }

    /// Sets the screen for deleting a note.
    ///
    /// # Errors
    /// Will fail if no note is selected.
    pub fn begin_delete_note(&mut self) -> Result<()> {
        let index = self.selected_index().context("No note selected")?;
        self.screen = Screen::DeleteNote { index };
        Ok(())
    }

    // Removes the last character in the `input`.
    pub fn pop_input(&mut self) {
        if let Some(input) = self.input_mut()
            && let Some((idx, _)) = input.grapheme_indices(true).next_back()
        {
            input.truncate(idx);
        }
    }

    /// Discards the current modal. Resetting back to `Screen::Main`.
    pub fn cancel(&mut self) {
        self.screen = Screen::Main;
    }

    /// Applies the current modal. Keeps it open on error so the user can fix input.
    pub fn confirm(&mut self) -> Result<()> {
        let result = match self.screen.clone() {
            Screen::Main => return Ok(()),
            Screen::NewNote { input } => self.create_note(input),
            Screen::RenameNote { index, input } => self.rename_note(index, input),
            Screen::DeleteNote { index } => self.delete_note(index),
        };
        if result.is_ok() {
            self.screen = Screen::Main;
        }
        result
    }

    /// Mutable access to the text buffer when a text prompt is open.
    pub fn input_mut(&mut self) -> Option<&mut String> {
        match &mut self.screen {
            Screen::NewNote { input } | Screen::RenameNote { input, .. } => Some(input),
            _ => None,
        }
    }

    /// Sets the `status` field to be displayed by the UI.
    pub fn set_status(&mut self, status: String) {
        self.status = Some(status);
    }

    /// Sets the `list_state`.
    pub fn set_list_state(&mut self, list_state: ListState) {
        self.list_state = list_state;
    }

    /// clears the `status`.
    pub fn clear_status(&mut self) {
        self.status = None;
    }

    /// Selects the next note entry.
    pub fn select_next_note(&mut self) {
        let len = self.notes.len();
        if len == 0 {
            self.list_state.select(None);
            return;
        }
        let next = self
            .list_state
            .selected()
            .map_or(0, |i| (i + 1).min(len - 1));
        self.list_state.select(Some(next));
        self.scroll.reset();
    }

    /// Selects the previous note entry.
    pub fn select_prev_note(&mut self) {
        let len = self.notes.len();
        if len == 0 {
            self.list_state.select(None);
            return;
        }
        let prev = self
            .list_state
            .selected()
            .map_or(0, |i| i.saturating_sub(1));
        self.list_state.select(Some(prev));
        self.scroll.reset();
    }

    /// Deletes a `Note` entry from notes and filesystem.
    fn delete_note(&mut self, index: usize) -> Result<()> {
        let Some(note) = self.notes.get(index) else {
            return Ok(());
        };
        fs::remove_file(&note.path)?;
        self.notes.remove(index);
        self.list_state.select(if self.notes.is_empty() {
            None
        } else {
            Some(index.min(self.notes.len() - 1))
        });
        self.scroll.reset();
        Ok(())
    }

    /// Renames the title of a `Note` entry and filename.
    fn rename_note(&mut self, index: usize, new_title: String) -> Result<()> {
        validate_title(&new_title)?;
        let old_note = self.notes.get(index).context("Note no longer exists")?;
        let old_title = old_note.title.clone();
        if new_title == old_title {
            return Ok(());
        }
        let old_path = old_note.path.clone();

        if self.notes.iter().any(|n| n.title == new_title) {
            bail!("Duplicate title \"{}\"", new_title);
        }

        let new_path = Path::new(&self.config.notes_dir).join(format!("{new_title}.md"));
        if new_path.exists() && new_path.canonicalize()? != old_path.canonicalize()? {
            bail!("Filename already exists \"{}.md\"", new_title);
        }

        fs::rename(&old_path, &new_path)?;

        self.notes[index].title = new_title.clone();
        self.notes[index].path = new_path;

        self.notes.sort_by_cached_key(|a| a.title.to_lowercase());
        self.list_state
            .select(self.notes.iter().position(|x| x.title == new_title));

        self.scroll.reset();
        Ok(())
    }

    /// Creates a new `Note` entry and file.
    fn create_note(&mut self, title: String) -> Result<()> {
        validate_title(&title)?;

        if self.notes.iter().any(|n| n.title == title) {
            bail!("Duplicate title \"{}\"", title);
        }

        let path = Path::new(&self.config.notes_dir).join(format!("{title}.md"));

        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .with_context(|| format!("{title}.md already exists"))?;

        let note = Note::from_path(path).with_context(|| "Failed to create note")?;

        self.notes.push(note);
        self.notes.sort_by_cached_key(|a| a.title.to_lowercase());
        self.list_state
            .select(self.notes.iter().position(|x| x.title == title));

        self.scroll.reset();
        Ok(())
    }
}

/// Validates title.
fn validate_title(title: &str) -> Result<()> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        bail!("Title cannot be empty");
    }
    if title != trimmed {
        bail!("Title cannot start or end with whitespace");
    }
    if title.contains(['/', '\\', '\0']) {
        bail!("Title contains invalid characters");
    }
    if cfg!(windows) {
        validate_title_windows(title)?
    }
    Ok(())
}

/// Windows-only title validation.
fn validate_title_windows(title: &str) -> Result<()> {
    const RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9", "CONIN$",
        "CONOUT$",
    ];

    if title
        .chars()
        .any(|c| c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*'))
    {
        bail!("Title contains characters not allowed on Windows");
    }
    if title.ends_with('.') || title.ends_with(' ') {
        bail!("Title cannot end with a dot or space on Windows");
    }

    let stem = title.split('.').next().unwrap_or(title);
    let stem = stem.trim_end_matches([' ', '.']).to_ascii_uppercase();
    if RESERVED.contains(&stem.as_str()) {
        bail!("\"{title}\" is a reserved name on Windows");
    }

    Ok(())
}

/// Current app screen.
#[derive(Clone, Debug, PartialEq)]
pub enum Screen {
    Main,
    NewNote { input: String },
    RenameNote { index: usize, input: String },
    DeleteNote { index: usize },
}

/// Scroll info.
#[derive(Debug, Default)]
pub struct Scroll {
    offset: u16,
    max: u16,
}

impl Scroll {
    /// Returns `offset`.
    pub fn offset(&self) -> u16 {
        self.offset
    }

    /// scrolls down `amount`.
    pub fn down(&mut self, amount: u16) {
        self.offset = self.offset.saturating_add(amount).min(self.max);
    }

    /// scrolls up `amount`.
    pub fn up(&mut self, amount: u16) {
        self.offset = self.offset.saturating_sub(amount);
    }

    /// Resets scroll offset.
    pub fn reset(&mut self) {
        self.offset = 0;
    }

    /// Reconcile with the measurements from the last layout pass.
    pub fn sync(&mut self, viewport_height: u16, content_height: u16) {
        self.max = content_height.saturating_sub(viewport_height);
        self.offset = self.offset.min(self.max);
    }
}
