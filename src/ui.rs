use crate::app::{App, Screen};

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, Padding, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
};
use tui_markdown::{BuiltinCodeTheme, Options, StyleSheet};

/// Default theme for `tui_markdown`
#[derive(Debug, Clone)]
struct MarkdownTheme;

impl StyleSheet for MarkdownTheme {
    fn heading(&self, _level: u8) -> Style {
        Style::new().bold().blue()
    }

    fn table_header(&self) -> Style {
        Style::new().bold().blue()
    }
}

/// Render the UI.
pub fn render(frame: &mut Frame, app: &mut App) {
    let layout = Layout::vertical([
        Constraint::Length(app.config.header.lines().count() as u16),
        Constraint::Fill(1),
        Constraint::Fill(3),
        Constraint::Length(1),
    ])
    .spacing(1)
    .split(frame.area());

    let title = Paragraph::new(Text::styled(&app.config.header, Style::default()));
    frame.render_widget(title.centered(), layout[0]);

    let guides = Line::from(vec![
        Span::styled(" Q ", Style::new().on_blue().black()),
        " Quit ".into(),
        Span::styled(" J ", Style::new().on_blue().black()),
        " Down ".into(),
        Span::styled(" K ", Style::new().on_blue().black()),
        " Up ".into(),
        Span::styled(" N ", Style::new().on_blue().black()),
        " New ".into(),
        Span::styled(" R ", Style::new().on_blue().black()),
        " Rename ".into(),
        Span::styled(" D ", Style::new().on_blue().black()),
        " Delete ".into(),
        Span::styled(" 󰌑 ", Style::new().on_blue().black()),
        " Edit ".into(),
    ]);
    frame.render_widget(guides.centered(), layout[3]);

    let list_area = layout[1].centered(Constraint::Ratio(3, 4), Constraint::Ratio(1, 2));
    render_list(frame, list_area, app);

    let preview_area = layout[2];
    render_preview(frame, preview_area, app);

    match app.current_screen {
        Screen::NewNote => render_input_popup(frame, frame.area(), app),
        Screen::RenameNote => render_input_popup(frame, frame.area(), app),
        Screen::DeleteNote => render_delete_popup(frame, frame.area(), app),
        _ => {}
    }
}

/// Render the list of notes.
pub fn render_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let items = app
        .notes
        .iter()
        .map(|note| {
            let title = note.0.to_string();
            ListItem::new(Line::from(title).centered())
        })
        .collect::<Vec<_>>();

    let list = List::new(items)
        .scroll_padding(5)
        .style(Color::Blue)
        .highlight_style(Modifier::REVERSED);

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

/// Render the previewer.
pub fn render_preview(frame: &mut Frame, area: Rect, app: &mut App) {
    let content = app
        .list_state
        .selected()
        .and_then(|index| app.notes.values().nth(index))
        .map(|note| note.content.as_str())
        .unwrap_or_default();

    let options = Options::new(MarkdownTheme).code_theme(BuiltinCodeTheme::InspiredGitHub);
    let formatted_text = tui_markdown::from_str_with_options(content, &options);
    let scroll_depth = formatted_text.height();
    app.scroll_offset.0 = app.scroll_offset.0.min(scroll_depth as u16);
    let preview = Paragraph::new(formatted_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .padding(Padding::horizontal(1)),
        )
        .wrap(Wrap { trim: false })
        .scroll(app.scroll_offset);

    frame.render_widget(preview, area);

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .style(Color::Blue);

    let mut scrollbar_state =
        ScrollbarState::new(scroll_depth).position(app.scroll_offset.0 as usize);

    frame.render_stateful_widget(
        scrollbar,
        area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        }),
        &mut scrollbar_state,
    );
}

pub fn render_input_popup(frame: &mut Frame, area: Rect, app: &mut App) {
    let guides = Line::from(vec![
        " ".into(),
        Span::styled(" Esc ", Style::new().on_blue().black()),
        " Quit ".into(),
        Span::styled(" 󰌑 ", Style::new().on_blue().black()),
        " Confirm ".into(),
    ])
    .centered();

    let title = match app.current_screen {
        Screen::NewNote => Span::styled(" New Note ", Style::new().blue()),
        Screen::RenameNote => Span::styled(" Rename Note ", Style::new().blue()),
        _ => panic!("Impossible to reach."),
    };

    let input = Line::from(app.input.clone()).style(Style::new().white());
    let popup = Paragraph::new(input).block(
        Block::default()
            .title(title)
            .title_bottom(guides)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue))
            .padding(Padding::uniform(1)),
    );

    let popup_layout = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(6),
        Constraint::Fill(1),
    ])
    .split(area);

    let popup_layout = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length((app.input.len() + 4).max(30) as u16),
        Constraint::Fill(1),
    ])
    .split(popup_layout[1])[1];

    frame.render_widget(Clear, popup_layout);
    frame.render_widget(popup, popup_layout);

    frame.set_cursor_position(Position::new(
        popup_layout.x + app.input.len() as u16 + 2,
        popup_layout.y + 2,
    ));
}

pub fn render_delete_popup(frame: &mut Frame, area: Rect, app: &mut App) {
    let guides = Line::from(vec![
        " ".into(),
        Span::styled(" N ", Style::new().on_blue().black()),
        " No ".into(),
        Span::styled(" Y ", Style::new().on_blue().black()),
        " Yes ".into(),
    ])
    .centered();

    let note_title = &app.current_note().unwrap().title;

    let prompt = Line::from(vec![
        Span::styled("Delete ", Style::new().white()),
        Span::styled(format!("\"{note_title}\""), Style::new().red()),
        Span::styled("?", Style::new().white()),
    ])
    .centered();

    let title = Span::styled(" Delete Note ", Style::new().blue());
    let popup = Paragraph::new(prompt).block(
        Block::default()
            .title(title)
            .title_bottom(guides)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue))
            .padding(Padding::uniform(1)),
    );

    let popup_layout = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(6),
        Constraint::Fill(1),
    ])
    .split(area);

    let popup_layout = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length((note_title.len() + 20).max(30) as u16),
        Constraint::Fill(1),
    ])
    .split(popup_layout[1])[1];

    frame.render_widget(Clear, popup_layout);
    frame.render_widget(popup, popup_layout);
}
