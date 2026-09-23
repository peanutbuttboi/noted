use crate::{config::Config, note::Note};

use anyhow::Result;
use ratatui::widgets::ListState;
use std::fs::read_dir;

/// State of the app.
#[derive(Debug)]
pub struct App {
    pub notes: Vec<Note>,
    pub list_state: ListState,
    pub config: Config,
}

impl App {
    /// Tries to build an instance of [`App`] with an instance of [`Config`]
    ///
    /// # Errors
    /// It will fail if the `notes_dir` doesn't exists.
    pub fn build(config: Config) -> Result<Self> {
        let dir = read_dir(&config.notes_dir)?;
        let mut notes = vec![];

        for entry in dir {
            let path = entry?.path();

            match Note::from_path(&path) {
                Ok(note) => notes.push(note),
                Err(_) => continue,
            }
        }

        notes.sort_by_key(|note| note.title.clone());

        Ok(Self {
            notes,
            list_state: ListState::default().with_selected(Some(0)),
            config,
        })
    }

    /// Selects the next note entry.
    pub fn select_next(&mut self) {
        let item_count = self.notes.len().saturating_sub(1);
        let next = self
            .list_state
            .selected()
            .map_or(0, |i| (i + 1).min(item_count));
        self.list_state.select(Some(next));
    }

    /// Selects the previous note entry.
    pub fn select_prev(&mut self) {
        let prev = self
            .list_state
            .selected()
            .map_or(0, |i| i.saturating_sub(1));
        self.list_state.select(Some(prev));
    }

    pub fn delete_note(&mut self) -> Result<()> {
        Ok(())
    }
}
