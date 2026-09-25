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

use std::{fs, io::stdout, path::Path, str::FromStr};

use anyhow::{Context, Result, bail};
use crossterm::execute;
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
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
                Action::Rename => {
                    if app.current_note().is_some() {
                        app.enter_screen(Screen::RenameNote);
                    } else {
                        app.set_status("No note selected".to_string());
                    }
                }
                Action::Delete => {
                    if app.current_note().is_some() {
                        app.enter_screen(Screen::DeleteNote);
                    } else {
                        app.set_status("No note selected".to_string());
                    }
                }

                Action::Edit => {
                    if app.current_note().is_some() {
                        if let Err(err) = run_editor(terminal, app) {
                            app.set_status(format!("{err:#}"));
                        }
                    } else {
                        app.set_status("No note selected".to_string());
                    }
                }
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
                Action::Confirm => {
                    if let Err(err) = app.save_input() {
                        app.set_status(format!("{err:#}"));
                    }
                }
                Action::Deny => app.enter_screen(Screen::Main),
                _ => {}
            },
            Screen::DeleteNote => match handle_events(app)? {
                Action::Confirm => {
                    if let Err(err) = app.delete_note() {
                        app.set_status(format!("{err:#}"));
                    }
                }
                Action::Deny => app.enter_screen(Screen::Main),
                _ => {}
            },
        }
    }
}

/// Run the editor to edit the note.
fn run_editor(terminal: &mut Terminal, app: &mut App) -> Result<()> {
    let Some(note) = app.current_note_mut() else {
        bail!("No note selected")
    };

    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    let edit_result = edit::edit_file(&note.path);
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    terminal.clear()?;
    edit_result?;

    note.update_content()?;
    Ok(())
}

fn parse_config() -> Result<Config> {
    let Some(config_dir) = dirs::config_dir() else {
        return Config::user_default();
    };

    let config_path = Path::new(&config_dir).join("noted").join("config.toml");

    if fs::exists(&config_path).context("Failed to check config file's existence.")? {
        Ok(Config::from_str(&fs::read_to_string(config_path)?)?)
    } else {
        Config::user_default()
    }
}
