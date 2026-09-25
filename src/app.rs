use crate::{config::Config, note::Note};

use ratatui::widgets::ListState;
use std::{
    fs::{self, OpenOptions, read_dir},
    path::Path,
};

use anyhow::{Context, Result, bail};

/// Current app screen
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Screen {
    Main,
    NewNote,
    DeleteNote,
    RenameNote,
}

/// App structure
#[derive(Debug)]
pub struct App {
    pub notes: Vec<Note>,
    pub config: Config,
    pub list_state: ListState,
    pub scroll_offset: u16,
    pub current_screen: Screen,
    pub input: String,
    pub status: Option<String>,
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
                Err(e) => eprintln!("Warning: Invalid file: {e}"),
            };
        }

        notes.sort_by_cached_key(|a| a.title.to_lowercase());

        let selected = if notes.is_empty() { None } else { Some(0) };

        Ok(Self {
            notes,
            config,
            list_state: ListState::default().with_selected(selected),
            scroll_offset: 0,
            current_screen: Screen::Main,
            input: String::new(),
            status: None,
        })
    }

    /// Returns a reference to the currently selected `Note` entry.
    ///
    /// Returns `None` if no note exists.
    pub fn current_note(&self) -> Option<&Note> {
        self.notes.get(self.list_state.selected()?)
    }

    /// Returns a mutable reference to the currently selected `Note` entry.
    ///
    /// Returns `None` if no note exists.
    pub fn current_note_mut(&mut self) -> Option<&mut Note> {
        self.notes.get_mut(self.list_state.selected()?)
    }

    pub fn set_status(&mut self, status: String) {
        self.status = Some(status);
    }

    /// Selects the next note entry.
    pub fn select_next(&mut self) {
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
        self.scroll_offset = 0;
    }

    /// Selects the previous note entry.
    pub fn select_prev(&mut self) {
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
        self.scroll_offset = 0;
    }

    pub fn set_scroll_offset(&mut self, offset: u16) {
        self.scroll_offset = offset;
    }

    /// Scrolls down by `amount`
    pub fn scroll_down(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
    }

    /// Scrolls up by `amount`
    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    /// Enters the respective `Screen`
    pub fn enter_screen(&mut self, screen: Screen) {
        self.current_screen = screen;
        match screen {
            Screen::NewNote => {
                self.input = String::new();
            }
            Screen::RenameNote => {
                let Some(note) = self.current_note() else {
                    self.current_screen = Screen::Main;
                    return;
                };

                self.input = note.title.to_string();
            }
            _ => {}
        }
    }

    pub fn save_input(&mut self) -> Result<()> {
        let title = self.input.clone();

        match self.current_screen {
            Screen::NewNote => {
                self.create_note(title)?;
            }
            Screen::RenameNote => {
                self.rename_note(title)?;
            }
            _ => {}
        }

        self.input = String::new();
        self.current_screen = Screen::Main;

        Ok(())
    }

    /// Deletes a `Note` entry from notes and filesystem.
    pub fn delete_note(&mut self) -> Result<()> {
        let Some(index) = self.list_state.selected() else {
            self.current_screen = Screen::Main;
            return Ok(());
        };
        let Some(note) = self.notes.get(index) else {
            self.current_screen = Screen::Main;
            return Ok(());
        };

        fs::remove_file(&note.path)?;
        self.notes.remove(index);

        let len = self.notes.len();
        self.list_state.select(if len == 0 {
            None
        } else {
            Some(index.min(len - 1))
        });

        self.scroll_offset = 0;
        self.current_screen = Screen::Main;
        Ok(())
    }

    /// Renames the title of a `Note` entry and filename.
    fn rename_note(&mut self, new_title: String) -> Result<()> {
        validate_title(&new_title)?;

        let Some(index) = self.list_state.selected() else {
            return Ok(());
        };
        let Some(note) = self.current_note() else {
            return Ok(());
        };

        let old_title = note.title.clone();
        if new_title == old_title {
            return Ok(());
        }
        let old_path = note.path.clone();

        if self.notes.iter().find(|n| n.title == new_title).is_some() {
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

        self.scroll_offset = 0;
        Ok(())
    }

    /// Creates a new `Note` entry and file.
    fn create_note(&mut self, title: String) -> Result<()> {
        validate_title(&title)?;

        if self.notes.iter().find(|n| n.title == title).is_some() {
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

        self.scroll_offset = 0;
        Ok(())
    }
}

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
    // optionally reject Windows-reserved names and trailing '.'
    Ok(())
}
