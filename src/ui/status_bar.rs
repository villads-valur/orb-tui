use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, InputMode};
use crate::ui::styles;

pub fn render_header(frame: &mut Frame, _app: &App, area: Rect) {
    let title = Line::from(vec![
        Span::styled(
            " orbstack-tui ",
            Style::default()
                .fg(styles::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("v0.1.0", Style::default().fg(styles::DIM_TEXT)),
    ]);

    frame.render_widget(Paragraph::new(title), area);
}

pub fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let line = match app.input_mode {
        InputMode::Search => Line::from(vec![
            Span::styled(
                " /",
                Style::default()
                    .fg(styles::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&app.search_query, Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(styles::ACCENT)),
        ]),
        InputMode::Confirm => {
            if let Some(msg) = &app.confirm_message {
                Line::from(Span::styled(
                    format!(" {msg}"),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                default_keybindings(app)
            }
        }
        InputMode::Normal => {
            if let Some(msg) = &app.message {
                Line::from(Span::styled(
                    format!(" {msg}"),
                    Style::default().fg(Color::Yellow),
                ))
            } else {
                default_keybindings(app)
            }
        }
    };

    let paragraph = Paragraph::new(line).style(Style::default().bg(styles::HIGHLIGHT_BG));
    frame.render_widget(paragraph, area);
}

fn default_keybindings(app: &App) -> Line<'static> {
    let mut bindings = vec![
        ("↑↓", "navigate"),
        ("Enter", "select"),
        ("Tab", "switch tab"),
        ("/", "search"),
    ];

    if !app.search_filter.is_empty() {
        bindings.push(("Esc", "clear filter"));
    }

    bindings.extend_from_slice(&[
        ("s", "start"),
        ("x", "stop"),
        ("R", "restart"),
        ("d", "remove"),
        ("q", "quit"),
    ]);

    let mut spans = vec![Span::raw(" ")];

    // Show active filter
    if !app.search_filter.is_empty() {
        spans.push(Span::styled(
            format!("[filter: {}]", app.search_filter),
            Style::default().fg(Color::Yellow),
        ));
        spans.push(Span::raw("  "));
    }

    for (i, (key, desc)) in bindings.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(
            *key,
            Style::default()
                .fg(styles::ACCENT)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(":{desc}"),
            Style::default().fg(Color::Gray),
        ));
    }

    Line::from(spans)
}
