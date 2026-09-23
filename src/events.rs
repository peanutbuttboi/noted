use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyModifiers};

/// The possible actions to be handled
pub enum Action {
    Up,
    Down,
    ScrollUp(u16),
    ScrollDown(u16),
    New,
    Rename,
    Edit,
    Delete,
    Quit,
    None,
}

/// Handles the incoming events.
///
/// Blocks until an event happens.
///
/// # Errors
/// It will fail if `crossterm::event` fails.
pub fn handle_events() -> Result<Action> {
    match event::read()? {
        Event::Key(key) => match (key.code, key.modifiers) {
            (KeyCode::Char('k'), KeyModifiers::NONE) => Ok(Action::Up),
            (KeyCode::Char('j'), KeyModifiers::NONE) => Ok(Action::Down),
            (KeyCode::Char('k'), KeyModifiers::CONTROL) => Ok(Action::ScrollUp(1)),
            (KeyCode::Char('j'), KeyModifiers::CONTROL) => Ok(Action::ScrollDown(1)),
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => Ok(Action::ScrollUp(23)),
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => Ok(Action::ScrollDown(23)),
            (KeyCode::Char('n'), KeyModifiers::NONE) => Ok(Action::New),
            (KeyCode::Char('r'), KeyModifiers::NONE) => Ok(Action::Rename),
            (KeyCode::Char('d'), KeyModifiers::NONE) => Ok(Action::Delete),
            (KeyCode::Char('q'), KeyModifiers::NONE) => Ok(Action::Quit),
            (KeyCode::Enter, KeyModifiers::NONE) => Ok(Action::Edit),
            _ => Ok(Action::None),
        },
        _ => Ok(Action::None),
    }
}
