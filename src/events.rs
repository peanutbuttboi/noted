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

const PAGE_SCROLL: u16 = 23;

/// Handles the incoming events.
///
/// Blocks until an event is received.
///
/// # Errors
/// It will fail if `crossterm::event::read` fails.
pub fn handle_events(screen: &Screen) -> Result<Action> {
    match event::read()? {
        Event::Key(key) if is_actionable(key.kind) => Ok(map_key(screen, key)),
        _ => Ok(Action::None),
    }
}

/// Checks if the key kind is actionable.
fn is_actionable(keykind: KeyEventKind) -> bool {
    matches!(keykind, KeyEventKind::Press | KeyEventKind::Repeat)
}

/// Maps key to its associated `Action`.
fn map_key(screen: &Screen, key: KeyEvent) -> Action {
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

#[cfg(test)]
mod test {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_is_actionable() {
        assert!(is_actionable(KeyEventKind::Press));
        assert!(is_actionable(KeyEventKind::Repeat));
        assert!(!is_actionable(KeyEventKind::Release));
    }

    #[test]
    fn test_map_key() {
        let cases = [
            // `Main` navigation
            (
                Screen::Main,
                KeyCode::Char('k'),
                KeyModifiers::NONE,
                Action::Up,
            ),
            (Screen::Main, KeyCode::Up, KeyModifiers::NONE, Action::Up),
            (
                Screen::Main,
                KeyCode::Char('j'),
                KeyModifiers::NONE,
                Action::Down,
            ),
            (
                Screen::Main,
                KeyCode::Down,
                KeyModifiers::NONE,
                Action::Down,
            ),
            // `Main` scrolling
            (
                Screen::Main,
                KeyCode::Char('k'),
                KeyModifiers::CONTROL,
                Action::ScrollUp(1),
            ),
            (
                Screen::Main,
                KeyCode::Char('j'),
                KeyModifiers::CONTROL,
                Action::ScrollDown(1),
            ),
            (
                Screen::Main,
                KeyCode::Char('u'),
                KeyModifiers::CONTROL,
                Action::ScrollUp(PAGE_SCROLL),
            ),
            (
                Screen::Main,
                KeyCode::Char('d'),
                KeyModifiers::CONTROL,
                Action::ScrollDown(PAGE_SCROLL),
            ),
            // `Main` commands
            (
                Screen::Main,
                KeyCode::Char('n'),
                KeyModifiers::NONE,
                Action::New,
            ),
            (
                Screen::Main,
                KeyCode::Char('r'),
                KeyModifiers::NONE,
                Action::Rename,
            ),
            (
                Screen::Main,
                KeyCode::Char('d'),
                KeyModifiers::NONE,
                Action::Delete,
            ),
            (
                Screen::Main,
                KeyCode::Char('q'),
                KeyModifiers::NONE,
                Action::Quit,
            ),
            (Screen::Main, KeyCode::Esc, KeyModifiers::NONE, Action::Quit),
            (
                Screen::Main,
                KeyCode::Enter,
                KeyModifiers::NONE,
                Action::Edit,
            ),
            // `Main` unmapped / wrong modifiers
            (
                Screen::Main,
                KeyCode::Char('x'),
                KeyModifiers::NONE,
                Action::None,
            ),
            (
                Screen::Main,
                KeyCode::Char('k'),
                KeyModifiers::SHIFT,
                Action::None,
            ),
            (
                Screen::Main,
                KeyCode::Char('q'),
                KeyModifiers::CONTROL,
                Action::None,
            ),
            // `NewNote` text entry
            (
                Screen::NewNote {
                    input: String::new(),
                },
                KeyCode::Char('a'),
                KeyModifiers::NONE,
                Action::Char('a'),
            ),
            (
                Screen::NewNote {
                    input: String::new(),
                },
                KeyCode::Char('A'),
                KeyModifiers::SHIFT,
                Action::Char('A'),
            ),
            (
                Screen::NewNote {
                    input: String::new(),
                },
                KeyCode::Enter,
                KeyModifiers::NONE,
                Action::Confirm,
            ),
            (
                Screen::NewNote {
                    input: String::new(),
                },
                KeyCode::Enter,
                KeyModifiers::SHIFT,
                Action::Confirm,
            ),
            (
                Screen::NewNote {
                    input: String::new(),
                },
                KeyCode::Esc,
                KeyModifiers::NONE,
                Action::Deny,
            ),
            (
                Screen::NewNote {
                    input: String::new(),
                },
                KeyCode::Esc,
                KeyModifiers::SHIFT,
                Action::Deny,
            ),
            (
                Screen::NewNote {
                    input: String::new(),
                },
                KeyCode::Backspace,
                KeyModifiers::NONE,
                Action::Delete,
            ),
            (
                Screen::NewNote {
                    input: String::new(),
                },
                KeyCode::Backspace,
                KeyModifiers::SHIFT,
                Action::Delete,
            ),
            // `NewNote` unmapped / wrong modifiers
            (
                Screen::NewNote {
                    input: String::new(),
                },
                KeyCode::Tab,
                KeyModifiers::NONE,
                Action::None,
            ),
            (
                Screen::NewNote {
                    input: String::new(),
                },
                KeyCode::Char('a'),
                KeyModifiers::CONTROL,
                Action::None,
            ),
            // `RenameNote` text entry
            (
                Screen::RenameNote {
                    index: 0,
                    input: String::new(),
                },
                KeyCode::Char('z'),
                KeyModifiers::NONE,
                Action::Char('z'),
            ),
            (
                Screen::RenameNote {
                    index: 0,
                    input: String::new(),
                },
                KeyCode::Enter,
                KeyModifiers::NONE,
                Action::Confirm,
            ),
            (
                Screen::RenameNote {
                    index: 0,
                    input: String::new(),
                },
                KeyCode::Esc,
                KeyModifiers::NONE,
                Action::Deny,
            ),
            (
                Screen::RenameNote {
                    index: 0,
                    input: String::new(),
                },
                KeyCode::Backspace,
                KeyModifiers::NONE,
                Action::Delete,
            ),
            // `DeleteNote` confirmation
            (
                Screen::DeleteNote { index: 0 },
                KeyCode::Char('y'),
                KeyModifiers::NONE,
                Action::Confirm,
            ),
            (
                Screen::DeleteNote { index: 0 },
                KeyCode::Enter,
                KeyModifiers::NONE,
                Action::Confirm,
            ),
            (
                Screen::DeleteNote { index: 0 },
                KeyCode::Char('n'),
                KeyModifiers::NONE,
                Action::Deny,
            ),
            (
                Screen::DeleteNote { index: 0 },
                KeyCode::Esc,
                KeyModifiers::NONE,
                Action::Deny,
            ),
            // `DeleteNote` unmapped / wrong modifiers
            (
                Screen::DeleteNote { index: 0 },
                KeyCode::Char('x'),
                KeyModifiers::NONE,
                Action::None,
            ),
            (
                Screen::DeleteNote { index: 0 },
                KeyCode::Char('n'),
                KeyModifiers::CONTROL,
                Action::None,
            ),
            (
                Screen::DeleteNote { index: 0 },
                KeyCode::Char('n'),
                KeyModifiers::SHIFT,
                Action::None,
            ),
            (
                Screen::DeleteNote { index: 0 },
                KeyCode::Char('y'),
                KeyModifiers::CONTROL,
                Action::None,
            ),
            (
                Screen::DeleteNote { index: 0 },
                KeyCode::Char('y'),
                KeyModifiers::SHIFT,
                Action::None,
            ),
        ];

        for (screen, code, modifiers, expected) in cases {
            let key = KeyEvent::new(code, modifiers);
            assert_eq!(
                map_key(&screen, key),
                expected,
                "screen={screen:?}, code={code:?}, modifiers={modifiers:?}"
            );
        }
    }
}
