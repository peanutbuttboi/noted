use crate::app::{App, Screen};
use crate::config::UI;

use ratatui::widgets::ListState;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Position, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, Padding, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
};
use tui_markdown::{BuiltinCodeTheme, Options, StyleSheet};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Default theme for `tui_markdown`
#[derive(Debug, Clone, Copy)]
struct MarkdownTheme {
    accent: Color,
}

impl StyleSheet for MarkdownTheme {
    fn heading(&self, _level: u8) -> Style {
        Style::new().bold().fg(self.accent)
    }
    fn heading_meta(&self) -> Style {
        Style::new().fg(self.accent)
    }
    fn link(&self) -> Style {
        Style::new()
            .fg(self.accent)
            .add_modifier(Modifier::UNDERLINED)
    }
    fn blockquote(&self) -> Style {
        Style::new().fg(self.accent)
    }
    fn table_header(&self) -> Style {
        Style::new().bold().fg(self.accent)
    }
    fn table_border(&self) -> Style {
        Style::new().fg(self.accent)
    }
}

/// Returns a StyleSheet Options.
fn markdown_options(ui: &UI) -> Options<MarkdownTheme> {
    Options::new(MarkdownTheme { accent: ui.accent })
        .code_theme(BuiltinCodeTheme::Base16EightiesDark)
}

/// The screen layout.
#[derive(Clone, Copy, Debug)]
pub struct ScreenLayout {
    pub header: Rect,
    pub list: Rect,
    pub preview: Rect,
    pub preview_inner: Rect,
    pub guides: Option<Rect>,
}

/// Builds an [`ScreenLayout`] instance from an `area` and `UI` options.
pub fn layout(area: Rect, ui: &UI) -> ScreenLayout {
    let mut constraints = vec![
        Constraint::Length(ui.header.lines().count() as u16),
        Constraint::Fill(1),
        Constraint::Fill(3),
        Constraint::Length(1),
    ];

    if !ui.show_guides {
        constraints.pop();
    };

    let layout = Layout::vertical(constraints).spacing(1).split(area);

    let preview_block = preview_block();
    let preview_inner = preview_block.inner(layout[2]);

    ScreenLayout {
        header: layout[0],
        list: layout[1],
        preview: layout[2],
        preview_inner,
        guides: layout.get(3).copied(),
    }
}

/// Preview info.
pub struct Preview<'a> {
    paragraph: Paragraph<'a>,
    inner: Rect,
    pub text_height: usize,
}

/// Creates a [`Preview`] instance.
pub fn prepare_preview<'a>(content: &'a str, ui: &UI, inner: Rect) -> Preview<'a> {
    let text = tui_markdown::from_str_with_options(content, &markdown_options(ui));
    let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
    let text_height = paragraph.line_count(inner.width);
    Preview {
        paragraph,
        inner,
        text_height,
    }
}

/// Creates the preview block.
fn preview_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .padding(Padding::horizontal(1))
}

/// Render the UI.
pub fn render(
    frame: &mut Frame,
    app: &App,
    list_state: &mut ListState,
    layout: ScreenLayout,
    preview: Preview<'_>,
) {
    let header = Paragraph::new(Text::styled(&app.config().ui.header, Style::default()));
    frame.render_widget(header.centered(), layout.header);

    if let Some(guides_layout) = layout.guides {
        let guides = Line::from(vec![
            Span::styled(" Q ", Style::default().bg(app.config().ui.accent).black()),
            " Quit ".into(),
            Span::styled(" J ", Style::default().bg(app.config().ui.accent).black()),
            " Down ".into(),
            Span::styled(" K ", Style::default().bg(app.config().ui.accent).black()),
            " Up ".into(),
            Span::styled(" N ", Style::default().bg(app.config().ui.accent).black()),
            " New ".into(),
            Span::styled(" R ", Style::default().bg(app.config().ui.accent).black()),
            " Rename ".into(),
            Span::styled(" D ", Style::default().bg(app.config().ui.accent).black()),
            " Delete ".into(),
            Span::styled(" 󰌑 ", Style::default().bg(app.config().ui.accent).black()),
            " Edit ".into(),
        ]);
        frame.render_widget(guides.centered(), guides_layout);
    }

    render_list(
        frame,
        layout
            .list
            .centered(Constraint::Ratio(3, 4), Constraint::Ratio(1, 2)),
        app,
        list_state,
    );

    render_preview(frame, layout.preview, app, preview);

    match app.screen() {
        Screen::NewNote { input } => {
            render_input_popup(frame, frame.area(), app, "New Note", input)
        }
        Screen::RenameNote { input, .. } => {
            render_input_popup(frame, frame.area(), app, "Rename Note", input)
        }
        Screen::DeleteNote { index } => render_delete_popup(frame, frame.area(), app, *index),
        _ => {}
    }

    if let Some(text) = app.status() {
        render_status(frame, frame.area(), app, text);
    }
}

/// Render the list of notes.
pub fn render_list(frame: &mut Frame, area: Rect, app: &App, list_state: &mut ListState) {
    let items = app
        .notes()
        .iter()
        .map(|note| ListItem::new(Line::from(note.title.as_str()).centered()))
        .collect::<Vec<_>>();

    let list = List::new(items)
        .scroll_padding(5)
        .style(app.config().ui.accent)
        .highlight_style(Modifier::REVERSED);

    frame.render_stateful_widget(list, area, list_state);
}

/// Render the previewer.
pub fn render_preview(frame: &mut Frame, area: Rect, app: &App, preview: Preview<'_>) {
    frame.render_widget(preview_block(), area);
    frame.render_widget(
        preview.paragraph.scroll((app.scroll_offset(), 0)),
        preview.inner,
    );

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .style(app.config().ui.accent);

    let viewport = preview.inner.height as usize;
    let max_offset = preview.text_height.saturating_sub(viewport);
    let mut state = ScrollbarState::new(max_offset).position(app.scroll_offset() as usize);
    frame.render_stateful_widget(
        scrollbar,
        area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        }),
        &mut state,
    );
}

/// Renders a popup to capture input.
pub fn render_input_popup(frame: &mut Frame, area: Rect, app: &App, title: &str, input: &str) {
    let guides = Line::from(vec![
        " ".into(),
        Span::styled(" Esc ", Style::default().bg(app.config().ui.accent).black()),
        " Quit ".into(),
        Span::styled(" 󰌑 ", Style::default().bg(app.config().ui.accent).black()),
        " Confirm ".into(),
    ])
    .centered();

    let title = Span::styled(
        format!(" {title} "),
        Style::default().fg(app.config().ui.accent),
    );

    let desired = (input.width() + 4).max(30);
    let max = (area.width as usize * 3 / 4).saturating_sub(2).max(8);
    let popup_width = desired.min(max) as u16;
    let wrap_width = (popup_width as usize).saturating_sub(4).max(1);

    let input = fit_to_width(input, wrap_width);

    let mut input_widget = vec![];
    for line in input.lines() {
        input_widget.push(Line::from(line).style(Style::new().white()));
    }

    let popup = Paragraph::new(input_widget).block(
        Block::default()
            .title(title)
            .title_bottom(guides)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.config().ui.accent))
            .padding(Padding::uniform(1)),
    );

    let popup_layout = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length((input.lines().count() + 4).max(5) as u16),
        Constraint::Fill(1),
    ])
    .split(area);

    let popup_layout = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(popup_width),
        Constraint::Fill(1),
    ])
    .split(popup_layout[1])[1];

    frame.render_widget(Clear, popup_layout);
    frame.render_widget(popup, popup_layout);

    let cursor_x = input.lines().next_back().map_or(0, |l| l.width()) as u16 + 2;
    let cursor_y = if input.is_empty() {
        1
    } else {
        input.lines().count()
    } as u16
        + 1;

    frame.set_cursor_position(Position::new(
        popup_layout.x + cursor_x,
        popup_layout.y + cursor_y,
    ));
}

/// Renders delete confirmation popup.
pub fn render_delete_popup(frame: &mut Frame, area: Rect, app: &App, index: usize) {
    let Some(note) = app.notes().get(index) else {
        return;
    };
    let note_title = note.title.as_str();

    let guides = Line::from(vec![
        " ".into(),
        Span::styled(" N ", Style::default().bg(app.config().ui.accent).black()),
        " No ".into(),
        Span::styled(" Y ", Style::default().bg(app.config().ui.accent).black()),
        " Yes ".into(),
    ])
    .centered();

    let prompt = Line::from(vec![
        Span::styled("Delete ", Style::new().white()),
        Span::styled(format!("\"{note_title}\""), Style::new().red()),
        Span::styled("?", Style::new().white()),
    ])
    .centered();

    let title = Span::styled(" Delete Note ", Style::default().fg(app.config().ui.accent));
    let popup = Paragraph::new(prompt).block(
        Block::default()
            .title(title)
            .title_bottom(guides)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.config().ui.accent))
            .padding(Padding::uniform(1)),
    );

    let popup_layout = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(5),
        Constraint::Fill(1),
    ])
    .split(area);

    let popup_layout = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length((note_title.width() + 20).max(30) as u16),
        Constraint::Fill(1),
    ])
    .split(popup_layout[1])[1];

    frame.render_widget(Clear, popup_layout);
    frame.render_widget(popup, popup_layout);
}

/// Renders status popup.
pub fn render_status(frame: &mut Frame, area: Rect, app: &App, status: &str) {
    let text = Line::from(status).centered().bold();
    let popup = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.config().ui.accent))
            .padding(Padding::uniform(1)),
    );

    let popup_layout = Layout::vertical([
        Constraint::Length(status.lines().count() as u16 + 4),
        Constraint::Fill(1),
    ])
    .split(area);

    let popup_layout = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(
            (status
                .lines()
                .max_by_key(|x| x.width())
                .unwrap_or_default()
                .width()
                + 4)
            .max(30) as u16,
        ),
    ])
    .split(popup_layout[0])[1];

    frame.render_widget(Clear, popup_layout);
    frame.render_widget(popup, popup_layout);
}

/// Fits text to width by separating it into multiple lines.
pub fn fit_to_width(text: &str, width: usize) -> String {
    assert!(width > 0, "width must be greater than zero");

    let mut result = String::with_capacity(text.len());
    let mut line_width = 0;

    for ch in text.chars() {
        let w = ch.width().unwrap_or(0);
        if ch == '\n' {
            result.push('\n');
            line_width = 0;
            continue;
        }

        if line_width == width {
            result.push('\n');
            line_width = 0;
        }

        result.push(ch);
        line_width += w;
    }

    result
}
