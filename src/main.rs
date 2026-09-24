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

use std::{env, fs::read_to_string, io::stdout, path::Path, str::FromStr};
use std::{fs, process::Command};

use anyhow::Result;
use ratatui::crossterm::{
    ExecutableCommand,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

type Terminal = ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>;

/// Parses the config and creates the app instance.
/// Then runs the main loop.
fn main() -> Result<()> {
    let config = parse_config()?;
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
    let editor = env::var("EDITOR").unwrap();

    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    let status = Command::new(editor).arg(&note.path).status();
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    terminal.clear()?;
    status?;

    note.update_content()?;
    Ok(())
}

fn parse_config() -> Result<Config> {
    let home_path = env::var("HOME");
    if home_path.is_err() {
        return Ok(Config::default());
    }

    let config_path = Path::new(&home_path.unwrap())
        .join(".config")
        .join("noted")
        .join("config.toml");

    if fs::exists(&config_path).expect("Can't check the existence of the config file.") {
        Ok(Config::from_str(&read_to_string(config_path).unwrap())?)
    } else {
        Ok(Config::default())
    }
}
