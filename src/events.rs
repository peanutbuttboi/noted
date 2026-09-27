use anyhow::Result;
use crossterm::event::{KeyEvent, KeyEventKind};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyModifiers};

use crate::app::Screen;

/// The possible actions to be handled
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Up,
    Down,
    ScrollUp(u16),
    ScrollDown(u16),
    New,
    Rename,
    Edit,
    Delete,
    Char(char),
    Confirm,
    Deny,
    Quit,
    None,
}

/// Handles the incoming events.
///
/// Blocks until an event is received.
///
/// # Errors
/// It will fail if `crossterm::event::read` fails.
pub fn handle_events(screen: &Screen) -> Result<Action> {
    match event::read()? {
        Event::Key(key) if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
            Ok(map_key(screen, key))
        }
        _ => Ok(Action::None),
    }
}

/// Maps key to its associated `Action`.
fn map_key(screen: &Screen, key: KeyEvent) -> Action {
    const PAGE_SCROLL: u16 = 23;

    match screen {
        Screen::Main => match (key.code, key.modifiers) {
            (KeyCode::Char('k'), KeyModifiers::NONE) | (KeyCode::Up, KeyModifiers::NONE) => {
                Action::Up
            }
            (KeyCode::Char('j'), KeyModifiers::NONE) | (KeyCode::Down, KeyModifiers::NONE) => {
                Action::Down
            }
            (KeyCode::Char('k'), KeyModifiers::CONTROL) => Action::ScrollUp(1),
            (KeyCode::Char('j'), KeyModifiers::CONTROL) => Action::ScrollDown(1),
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => Action::ScrollUp(PAGE_SCROLL),
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => Action::ScrollDown(PAGE_SCROLL),
            (KeyCode::Char('n'), KeyModifiers::NONE) => Action::New,
            (KeyCode::Char('r'), KeyModifiers::NONE) => Action::Rename,
            (KeyCode::Char('d'), KeyModifiers::NONE) => Action::Delete,
            (KeyCode::Char('q'), KeyModifiers::NONE) | (KeyCode::Esc, KeyModifiers::NONE) => {
                Action::Quit
            }
            (KeyCode::Enter, KeyModifiers::NONE) => Action::Edit,
            _ => Action::None,
        },

        Screen::NewNote { .. } | Screen::RenameNote { .. }
            if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT =>
        {
            match key.code {
                KeyCode::Enter => Action::Confirm,
                KeyCode::Esc => Action::Deny,
                KeyCode::Backspace => Action::Delete,
                KeyCode::Char(k) => Action::Char(k),
                _ => Action::None,
            }
        }

        Screen::DeleteNote { .. } if key.modifiers == KeyModifiers::NONE => match key.code {
            KeyCode::Char('y') | KeyCode::Enter => Action::Confirm,
            KeyCode::Char('n') | KeyCode::Esc => Action::Deny,
            _ => Action::None,
        },
        _ => Action::None,
    }
}
