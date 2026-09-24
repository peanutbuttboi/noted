use crate::{config::Config, note::Note};

use ratatui::widgets::ListState;
use std::{
    collections::BTreeMap,
    fs::{self, read_dir},
    path::Path,
};

use anyhow::Result;

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
    pub notes: BTreeMap<String, Note>,
    pub config: Config,
    pub list_state: ListState,
    pub scroll_offset: (u16, u16),
    pub current_screen: Screen,
    pub input: String,
}

impl App {
    /// Tries to build an instance of [`App`] with an instance of [`Config`]
    ///
    /// # Errors
    /// It will fail if the `notes_dir` doesn't exists.
    pub fn build(config: Config) -> Result<Self> {
        let dir = read_dir(&config.notes_dir)?;
        let mut notes = BTreeMap::new();

        for entry in dir {
            let path = entry?.path();

            match Note::from_path(&path) {
                Ok(note) => notes.insert(note.title.clone(), note),
                Err(_) => continue,
            };
        }

        Ok(Self {
            notes,
            config,
            list_state: ListState::default().with_selected(Some(0)),
            scroll_offset: (0, 0),
            current_screen: Screen::Main,
            input: String::new(),
        })
    }

    /// Returns a refrence to the currently selected `Note` entry.
    ///
    /// Returns `None` if not found.
    pub fn current_note(&mut self) -> Option<&Note> {
        let index = self.list_state.selected()?;
        self.notes.values().nth(index)
    }

    /// Returns a mutable refrence to the currently selected `Note` entry.
    ///
    /// Returns `None` if not found.
    pub fn current_note_mut(&mut self) -> Option<&mut Note> {
        let index = self.list_state.selected()?;
        self.notes.values_mut().nth(index)
    }

    /// Selects the next note entry.
    pub fn select_next(&mut self) {
        let item_count = self.notes.len().saturating_sub(1);
        let next = self
            .list_state
            .selected()
            .map_or(0, |i| (i + 1).min(item_count));
        self.list_state.select(Some(next));
        self.scroll_offset.0 = 0;
    }

    /// Selects the previous note entry.
    pub fn select_prev(&mut self) {
        let prev = self
            .list_state
            .selected()
            .map_or(0, |i| i.saturating_sub(1));
        self.list_state.select(Some(prev));
        self.scroll_offset.0 = 0;
    }

    /// Scrolls down by `amount`
    pub fn scroll_down(&mut self, amount: u16) {
        self.scroll_offset.0 = self.scroll_offset.0.saturating_add(amount);
    }

    /// Scrolls up by `amount`
    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll_offset.0 = self.scroll_offset.0.saturating_sub(amount);
    }

    /// Enters the respective `Screen`
    pub fn enter_screen(&mut self, screen: Screen) {
        self.current_screen = screen;
        match screen {
            Screen::NewNote => {
                self.input = "".to_string();
            }
            Screen::RenameNote => {
                let note_title = &self.current_note().unwrap().title;
                self.input = note_title.to_string();
            }
            _ => {}
        }
    }

    pub fn save_input(&mut self) -> Result<()> {
        match self.current_screen {
            Screen::NewNote => {
                let title = self.input.clone();
                self.create_note(title)?;
                self.input = String::new();
            }
            Screen::RenameNote => {
                let note = self.current_note().unwrap();
                let title = note.title.clone();
                let mut value = self.notes.remove(&title).unwrap();
                let title = self.input.clone();
                value.title = title.clone();
                self.notes.insert(title, value);
                self.input = String::new();
            }
            _ => {}
        }

        self.current_screen = Screen::Main;

        Ok(())
    }

    /// Deletes a `Note` entry from notes and filesystem.
    pub fn delete_note(&mut self) -> Result<()> {
        let note = self.current_note().unwrap();
        let title = note.title.clone();
        fs::remove_file(&note.path)?;
        self.notes.remove(&title);
        self.current_screen = Screen::Main;
        Ok(())
    }

    /// Creates a new `Note` entry.
    pub fn create_note(&mut self, title: impl AsRef<str>) -> Result<()> {
        let path = Path::new(&self.config.notes_dir).join(format!("{}.md", title.as_ref()));
        fs::write(&path, "")?;
        let note = Note::from_path(path)?;
        self.notes.insert(note.title.clone(), note);
        self.list_state
            .select(self.notes.iter().position(|x| x.0 == title.as_ref()));
        self.current_screen = Screen::Main;
        Ok(())
    }
}
