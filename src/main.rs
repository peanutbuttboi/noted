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
        // `ListState.offset` is owned by the widget and is reconliced after drawing.
        let mut list_state = *app.list_state();
        let mut sync: Option<(u16, usize)> = None;

        terminal.draw(|frame| {
            let layout = ui::layout(frame.area(), &app.config().ui);
            let content = if let Some(index) = app.selected_index() {
                &app.notes()[index].content
            } else {
                ""
            };

            let preview = ui::prepare_preview(content, &app.config().ui, layout.preview_inner);

            sync = Some((layout.preview_inner.height, preview.text_height));

            ui::render(frame, app, &mut list_state, layout, preview);
        })?;

        if let Some((viewport, content)) = sync {
            app.sync_scroll(viewport, content.min(u16::MAX as usize) as u16);
        }

        app.set_list_state(list_state);
        app.clear_status();

        match app.screen() {
            Screen::Main => match handle_events(app.screen())? {
                Action::Down => app.select_next_note(),
                Action::Up => app.select_prev_note(),
                Action::ScrollUp(amount) => app.scroll_up(amount),
                Action::ScrollDown(amount) => app.scroll_down(amount),
                Action::New => app.begin_new_note(),
                Action::Rename => status_on_err(app, |a| a.begin_rename_note()),
                Action::Delete => status_on_err(app, |a| a.begin_delete_note()),
                Action::Edit => {
                    if app.selected_index().is_some() {
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
            Screen::NewNote { .. } | Screen::RenameNote { .. } => {
                match handle_events(app.screen())? {
                    Action::Char(c) => {
                        if let Some(input) = app.input_mut() {
                            input.push(c);
                        }
                    }
                    Action::Delete => app.pop_input(),
                    Action::Confirm => status_on_err(app, |a| a.confirm()),
                    Action::Deny => app.cancel(),
                    _ => {}
                }
            }
            Screen::DeleteNote { .. } => match handle_events(app.screen())? {
                Action::Confirm => status_on_err(app, |a| a.confirm()),
                Action::Deny => app.cancel(),
                _ => {}
            },
        }
    }
}

/// Call `set_status` in case of an Error.
fn status_on_err<F>(app: &mut App, f: F)
where
    F: FnOnce(&mut App) -> Result<()>,
{
    let result = f(app);

    match result {
        Ok(()) => {}
        Err(err) => app.set_status(format!("{err:#}")),
    }
}

/// Run the editor to edit the note.
fn run_editor(terminal: &mut Terminal, app: &mut App) -> Result<()> {
    let Some(index) = app.selected_index() else {
        bail!("No note selected")
    };
    let note = app.note_mut(index);

    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    let edit_result = edit::edit_file(&note.path);
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    terminal.clear()?;
    note.update_content()?;
    edit_result?;

    Ok(())
}

fn parse_config() -> Result<Config> {
    let Some(config_dir) = dirs::config_dir() else {
        return Config::user_default();
    };

    let config_path = Path::new(&config_dir).join("noted").join("config.toml");

    if fs::exists(&config_path).with_context(|| {
        format!(
            "failed to check config file's existence {}",
            config_path.display()
        )
    })? {
        Ok(Config::from_str(
            &fs::read_to_string(&config_path)
                .with_context(|| format!("failed to read {} to string", config_path.display()))?,
        )
        .context("failed to build config from the contents.")?)
    } else {
        Config::user_default()
    }
}
