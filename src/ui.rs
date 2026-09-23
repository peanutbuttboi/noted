use crate::app::App;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, List, ListItem, Padding, Paragraph, Scrollbar, ScrollbarOrientation,
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
pub fn render(frame: &mut Frame, app: &mut App, scroll_offset: (u16, u16)) {
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
    render_preview(frame, preview_area, app, scroll_offset);
}

/// Render the list of notes.
pub fn render_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let items = app
        .notes
        .iter()
        .map(|note| {
            let title = String::from(&note.title);
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
pub fn render_preview(frame: &mut Frame, area: Rect, app: &mut App, scroll_offset: (u16, u16)) {
    let content = app
        .list_state
        .selected()
        .and_then(|index| app.notes.get(index))
        .map(|note| note.content.as_str())
        .unwrap_or_default();

    let options = Options::new(MarkdownTheme).code_theme(BuiltinCodeTheme::InspiredGitHub);
    let formatted_text = tui_markdown::from_str_with_options(content, &options);
    let preview = Paragraph::new(formatted_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .padding(Padding::horizontal(2)),
        )
        .wrap(Wrap { trim: false })
        .scroll(scroll_offset);

    frame.render_widget(preview, area);

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .style(Color::Blue);

    let mut scrollbar_state =
        ScrollbarState::new(content.lines().count()).position(scroll_offset.0 as usize);

    frame.render_stateful_widget(
        scrollbar,
        area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        }),
        &mut scrollbar_state,
    );
}
