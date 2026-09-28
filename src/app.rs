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

/// App state machine.
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
                Err(
                    NoteError::NotMarkdown(_) | NoteError::NotAFile(_) | NoteError::NonExistent(_),
                ) => {}
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
            .with_context(|| format!("Filename already exists \"{}.md\"", title))?;

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

#[cfg(test)]
mod test {
    use std::assert_matches;
    use std::fs::write;

    use super::*;
    use crate::config::UI;

    #[test]
    fn test_app_build_loads_and_sorts() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("bravo.md"), "b").unwrap();
        fs::write(dir.path().join("Alpha.md"), "a").unwrap();
        fs::write(dir.path().join("ignore.txt"), "x").unwrap();
        fs::create_dir(dir.path().join("sub.md")).unwrap();

        let app = app_in(dir.path());
        let titles: Vec<_> = app.notes().iter().map(|n| n.title.as_str()).collect();

        assert_eq!(titles, ["Alpha", "bravo"]);
        assert_eq!(app.selected_index(), Some(0));
    }

    #[test]
    fn test_app_build_creates_missing_dir() {
        let dir = tempfile::tempdir().unwrap();
        let notes = dir.path().join("does/not/exist");
        let app = App::build(Config {
            notes_dir: notes.clone(),
            ui: UI::default(),
        })
        .unwrap();

        assert!(notes.is_dir());
        assert!(app.notes().is_empty());
    }

    #[test]
    fn test_scroll() {
        let mut scroll = Scroll::default();
        scroll.sync(2, 8);

        scroll.up(16);
        assert_eq!(scroll.offset(), 0);

        scroll.down(16);
        assert_eq!(scroll.offset(), 6);

        scroll.sync(2, 4);
        assert_eq!(scroll.offset(), 2);

        scroll.reset();
        assert_eq!(scroll.offset(), 0);

        scroll.up(16);
        scroll.sync(4, 2);
        assert_eq!(scroll.offset(), 0);
    }

    #[test]
    fn test_validate_title() {
        let titles_valid = ["notes", "My Note", "note-1"];

        for title in titles_valid {
            assert!(validate_title(title).is_ok(), "expected {title:?} valid");
        }

        let titles_invalid = ["", " ", " x", "x ", "a/b", "a\\b", "a\0b"];

        for title in titles_invalid {
            assert!(validate_title(title).is_err(), "expected {title:?} invalid");
        }
    }

    #[test]
    fn test_validate_title_windows() {
        let titles_valid = ["CONcert", "COM10", "auxiliary", "my.note"];

        for title in titles_valid {
            assert!(
                validate_title_windows(title).is_ok(),
                "expected {title:?} valid"
            );
        }

        let titles_invalid = [
            "CON", "con", "CON.md", "aux", "COM1", "LPT9", "nul", "CONIN$", "foo.", "foo<bar",
            "a:b", "LPT1", "PRN", "CONOUT$", "foo ",
        ];

        for title in titles_invalid {
            assert!(
                validate_title_windows(title).is_err(),
                "expected {title:?} invalid"
            );
        }
    }
    #[test]
    fn test_app_rejects_invalid_title() {
        let note_dir = tempfile::tempdir().unwrap();
        let mut app = app_in(note_dir.path());

        // Create
        let err = app_new_note(&mut app, "a/b").unwrap_err();
        assert!(err.to_string().contains("invalid characters"));
        assert_matches!(app.screen(), Screen::NewNote { .. });
        assert!(app.notes().is_empty());
        assert!(!note_dir.path().join("a").exists());

        // Rename
        app_new_note(&mut app, "note").unwrap();
        let err = app_rename_note(&mut app, "").unwrap_err();
        assert!(err.to_string().contains("empty"));
        assert_matches!(app.screen(), Screen::RenameNote { .. });
        assert_eq!(app.notes()[0].title, "note");
        assert!(note_dir.path().join("note.md").exists());
    }

    fn app_in(dir: &Path) -> App {
        App::build(Config {
            notes_dir: dir.to_path_buf(),
            ui: UI::default(),
        })
        .unwrap()
    }

    fn app_new_note(app: &mut App, title: &str) -> Result<()> {
        app.begin_new_note();
        if let Some(input) = app.input_mut() {
            *input = String::from(title);
        }
        app.confirm()
    }

    fn app_rename_note(app: &mut App, new_title: &str) -> Result<()> {
        app.begin_rename_note()?;
        if let Some(input) = app.input_mut() {
            *input = String::from(new_title);
        }
        app.confirm()
    }

    fn app_delete_note(app: &mut App) -> Result<()> {
        app.begin_delete_note()?;
        app.confirm()
    }

    #[test]
    fn test_app_create_note() {
        let note_dir = tempfile::tempdir().unwrap();
        let mut app = app_in(note_dir.path());

        // Empty
        assert_eq!(app.notes(), &[]);
        assert_eq!(app.selected_index(), None);

        app_new_note(&mut app, "note").unwrap();
        let files_path = note_dir.path().join("note.md");
        let index = app.selected_index().unwrap();

        // File created
        assert!(files_path.exists());
        // Inserted and selected
        assert_eq!(app.notes()[index].title, "note");

        app_new_note(&mut app, "abc").unwrap();
        let index = app.selected_index().unwrap();

        // Inserted, sorted and selected
        assert_eq!(index, 0);
        assert_eq!(app.notes()[index].title, "abc");

        let err = app_new_note(&mut app, "abc").unwrap_err();

        // Duplicate title error
        assert!(err.to_string().contains("Duplicate title"));
        // Modal stays open
        assert_matches!(app.screen(), Screen::NewNote { .. });

        write(note_dir.path().join("taken.md"), "").unwrap();
        let err = app_new_note(&mut app, "taken").unwrap_err();

        // Existing file
        assert!(err.to_string().contains("Filename already exists"));
    }

    #[test]
    fn test_app_rename_note() {
        let note_dir = tempfile::tempdir().unwrap();
        let mut app = app_in(note_dir.path());

        let result = app_rename_note(&mut app, "title");
        // Renaming on empty notes
        assert!(result.is_err());

        app_new_note(&mut app, "note").unwrap();
        app_rename_note(&mut app, "renamed").unwrap();

        let index = app.selected_index().unwrap();

        // File renamed
        assert!(note_dir.path().join("renamed.md").exists());
        // Old file removed
        assert!(!note_dir.path().join("note.md").exists());
        // Title renamed and selected
        assert_eq!(app.notes()[index].title, "renamed");

        let result = app_rename_note(&mut app, "renamed");
        // Same title accepted
        assert!(result.is_ok());

        let result = app_rename_note(&mut app, "Renamed");
        // Case sensitive accepted
        assert!(result.is_ok());

        app_new_note(&mut app, "note").unwrap();
        let err = app_rename_note(&mut app, "Renamed").unwrap_err();

        // Duplicate title error
        assert!(err.to_string().contains("Duplicate title"));
        // Modal stays open
        assert_matches!(app.screen(), Screen::RenameNote { .. });

        fs::write(note_dir.path().join("taken.md"), "").unwrap();
        let err = app_rename_note(&mut app, "taken").unwrap_err();

        // Existing file
        assert!(err.to_string().contains("Filename already exists"));
    }

    #[test]
    fn test_app_delete_note() {
        let note_dir = tempfile::tempdir().unwrap();
        let mut app = app_in(note_dir.path());

        let result = app_delete_note(&mut app);
        // Deleting on empty notes
        assert!(result.is_err());

        app_new_note(&mut app, "note1").unwrap();
        app_new_note(&mut app, "note2").unwrap();
        app_new_note(&mut app, "note3").unwrap();

        app.select_prev_note();
        app_delete_note(&mut app).unwrap();

        let files_path = note_dir.path().join("note2.md");

        // Correct note deleted
        assert_eq!(
            app.notes()
                .iter()
                .map(|n| n.title.as_str())
                .collect::<Vec<_>>(),
            ["note1", "note3"]
        );
        // File deleted
        assert!(!files_path.exists());
        // Selection
        assert_eq!(app.selected_index(), Some(1));

        app_delete_note(&mut app).unwrap();
        app_delete_note(&mut app).unwrap();

        // Notes emptied
        assert!(app.notes().is_empty());
    }

    #[test]
    fn test_select_note() {
        let note_dir = tempfile::tempdir().unwrap();
        let mut app = app_in(note_dir.path());

        app.select_next_note();
        // Selection with no notes
        assert_eq!(app.selected_index(), None);

        app.select_prev_note();
        // Selection with no notes
        assert_eq!(app.selected_index(), None);

        app_new_note(&mut app, "note1").unwrap();
        app_new_note(&mut app, "note2").unwrap();

        app.select_next_note();
        // Selection at bounds
        assert_eq!(app.selected_index(), Some(1));

        app.sync_scroll(10, 20);
        app.scroll_down(5);

        app.select_next_note();
        // Normal selection
        assert_eq!(app.selected_index(), Some(1));
        // Scroll reset
        assert_eq!(app.scroll_offset(), 0);

        app.sync_scroll(10, 20);
        app.scroll_down(5);

        app.select_prev_note();
        // Normal selection
        assert_eq!(app.selected_index(), Some(0));
        // Scroll reset
        assert_eq!(app.scroll_offset(), 0);
    }

    #[test]
    fn test_status() {
        let note_dir = tempfile::tempdir().unwrap();
        let mut app = app_in(note_dir.path());

        assert_eq!(app.status(), &None);
        app.set_status("test".to_string());
        assert_eq!(app.status(), &Some("test".to_string()));
        app.clear_status();
        assert_eq!(app.status(), &None);
    }

    #[test]
    fn test_cancel() {
        let note_dir = tempfile::tempdir().unwrap();
        let mut app = app_in(note_dir.path());

        app.begin_new_note();
        assert_matches!(app.screen(), Screen::NewNote { .. });
        app.cancel();
        assert_matches!(app.screen(), Screen::Main);
    }

    #[test]
    fn test_input_mut() {
        let note_dir = tempfile::tempdir().unwrap();
        let mut app = app_in(note_dir.path());

        assert!(app.input_mut().is_none());
        app.begin_new_note();
        assert!(app.input_mut().is_some());
    }

    #[test]
    fn test_pop_input() {
        let note_dir = tempfile::tempdir().unwrap();
        let mut app = app_in(note_dir.path());

        app_new_note(&mut app, "noted").unwrap();
        app.begin_rename_note().unwrap();
        app.pop_input();
        assert_eq!(app.input_mut().unwrap(), "note");

        app_new_note(&mut app, "e\u{301}").unwrap();
        app.begin_rename_note().unwrap();
        app.pop_input();
        assert_eq!(app.input_mut().unwrap(), "");
    }
}
