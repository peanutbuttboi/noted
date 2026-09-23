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
    app::App,
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
    let mut scroll_offset = (0u16, 0u16);
    loop {
        terminal.draw(|frame| ui::render(frame, app, scroll_offset))?;
        match handle_events()? {
            Action::Down => {
                app.select_next();
                scroll_offset.0 = 0;
            }
            Action::Up => {
                app.select_prev();
                scroll_offset.0 = 0;
            }
            Action::ScrollUp(amount) => scroll_offset.0 = scroll_offset.0.saturating_sub(amount),
            Action::ScrollDown(amount) => scroll_offset.0 = scroll_offset.0.saturating_add(amount),
            Action::Edit => run_editor(terminal, app)?,
            Action::Quit => break Ok(()),
            _ => {}
        }
    }
}

/// Run the editor to edit the note.
fn run_editor(terminal: &mut Terminal, app: &mut App) -> Result<()> {
    let Some(index) = app.list_state.selected() else {
        return Ok(());
    };
    let Some(path) = app.notes.get(index).map(|note| &note.path) else {
        return Ok(());
    };

    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    let status = Command::new("nvim").arg(path).status();
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    terminal.clear()?;
    status?;

    if let Some(note) = app.notes.get_mut(index) {
        note.update_content()?;
    }
    Ok(())
}
