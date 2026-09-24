/// The state of the app.
mod app;

/// Note storage and handling.
mod note;

/// Event handling.
mod events;

/// UI drawing.
mod ui;

/// Config parsing and handling.
mod config;

use crate::{
    app::{App, Screen},
    config::Config,
    events::{Action, handle_events},
};

use std::io::stdout;
use std::process::Command;

use anyhow::Result;
use ratatui::crossterm::{
    ExecutableCommand,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

type Terminal = ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>;

/// Parses the config and creates the app instance.
/// Then runs the main loop.
fn main() -> Result<()> {
    let config = Config::default();
    let mut app = App::build(config)?;
    let mut terminal = ratatui::init();

    let result = run(&mut terminal, &mut app);

    ratatui::restore();
    result
}

/// Run the main drawing and event handling loop.
fn run(terminal: &mut Terminal, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;
        match app.current_screen {
            Screen::Main => match handle_events(app)? {
                Action::Down => app.select_next(),
                Action::Up => app.select_prev(),
                Action::ScrollUp(amount) => app.scroll_up(amount),
                Action::ScrollDown(amount) => app.scroll_down(amount),
                Action::New => app.enter_screen(Screen::NewNote),
                Action::Rename => app.enter_screen(Screen::RenameNote),
                Action::Delete => app.enter_screen(Screen::DeleteNote),
                Action::Edit => run_editor(terminal, app)?,
                Action::Quit => break Ok(()),
                _ => {}
            },
            Screen::NewNote | Screen::RenameNote => match handle_events(app)? {
                Action::Char(c) => {
                    app.input.push(c);
                }
                Action::Delete => {
                    app.input.pop();
                }
                Action::Confirm => app.save_input()?,
                Action::Deny => app.enter_screen(Screen::Main),
                _ => {}
            },
            Screen::DeleteNote => match handle_events(app)? {
                Action::Confirm => app.delete_note()?,
                Action::Deny => app.enter_screen(Screen::Main),
                _ => {}
            },
        }
    }
}

/// Run the editor to edit the note.
fn run_editor(terminal: &mut Terminal, app: &mut App) -> Result<()> {
    let note = app.current_note_mut().unwrap();

    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    let status = Command::new("nvim").arg(&note.path).status();
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    terminal.clear()?;
    status?;

    note.update_content()?;
    Ok(())
}
