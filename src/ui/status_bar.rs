use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, InputMode};
use crate::ui::styles;

pub fn render_header(frame: &mut Frame, _app: &App, area: Rect) {
    let title = Line::from(vec![
        Span::styled(
            " orb-tui ",
            Style::default()
                .fg(styles::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("v0.1.0", Style::default().fg(styles::DIM_TEXT)),
    ]);

    frame.render_widget(Paragraph::new(title), area);
}

pub fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let left_line = match app.input_mode {
        InputMode::Search => Line::from(vec![
            Span::styled(
                " /",
                Style::default()
                    .fg(styles::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&app.search_query, Style::default().fg(styles::TEXT)),
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

    let paragraph = Paragraph::new(left_line).style(styles::statusbar_style());
    frame.render_widget(paragraph, area);

    if app.busy_count > 0 {
        let spinner = Line::from(Span::styled(
            format!("{} ", app.spinner_frame()),
            Style::default().fg(styles::ACCENT),
        ));
        let right = Paragraph::new(spinner).alignment(Alignment::Right);
        frame.render_widget(right, area);
    }
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

    if app.fullscreen_detail {
        bindings.push(("z/Esc", "exit zoom"));
    } else {
        bindings.push(("z", "zoom"));
    }

    bindings.extend_from_slice(&[
        ("s", "start"),
        ("x", "stop"),
        ("r", "restart"),
        ("d", "remove"),
        ("q", "quit"),
    ]);

    let mut spans = vec![Span::raw(" ")];

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
