use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::{App, FocusedPanel, ListItem as AppListItem};
use crate::ui::styles;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focused_panel == FocusedPanel::ContainerList;

    if !app.docker_available {
        let block = Block::default()
            .title(" Containers ")
            .borders(Borders::ALL)
            .border_style(styles::border_style(focused));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = if app.orbstack_starting {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Starting OrbStack...",
                    Style::default()
                        .fg(styles::ACCENT)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Waiting for Docker to become available",
                    Style::default().fg(styles::DIM_TEXT),
                )),
            ]
        } else {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "OrbStack is not running",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Press ", Style::default().fg(styles::DIM_TEXT)),
                    Span::styled(
                        "o",
                        Style::default()
                            .fg(styles::ACCENT)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" to start OrbStack", Style::default().fg(styles::DIM_TEXT)),
                ]),
            ]
        };

        let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
        frame.render_widget(paragraph, inner);
        return;
    }

    let running = app.running_count();
    let total = app.containers.len();

    let title = format!(" Containers ({running}/{total}) ");
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(styles::border_style(focused));

    let items: Vec<ListItem> = app
        .list_items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let is_selected = idx == app.selected_index;
            match item {
                AppListItem::GroupHeader {
                    project,
                    expanded,
                    running_count,
                    total_count,
                } => {
                    let arrow = if *expanded { "▼" } else { "▶" };
                    let count_str = format!(" ({running_count}/{total_count})");
                    let marker = if is_selected { "▌" } else { " " };

                    let line = Line::from(vec![
                        Span::styled(marker, Style::default().fg(styles::ACCENT)),
                        Span::styled(format!("{arrow} "), Style::default().fg(styles::DIM_TEXT)),
                        Span::styled(
                            project.as_str(),
                            if is_selected {
                                styles::selected_style()
                            } else {
                                styles::group_header_style()
                            },
                        ),
                        Span::styled(count_str, Style::default().fg(styles::DIM_TEXT)),
                    ]);

                    ListItem::new(line)
                }
                AppListItem::Container { index } => {
                    let container = &app.containers[*index];
                    let state_color = styles::container_state_color(&container.state);
                    let symbol = container.state.symbol();
                    let display_name = container.display_name();
                    let marker = if is_selected { "▌" } else { " " };

                    let name_style = if is_selected {
                        styles::selected_style()
                    } else {
                        Style::default().fg(styles::TEXT)
                    };

                    let line = Line::from(vec![
                        Span::styled(marker, Style::default().fg(styles::ACCENT)),
                        Span::raw(" "),
                        Span::styled(format!("{symbol} "), Style::default().fg(state_color)),
                        Span::styled(display_name, name_style),
                    ]);

                    let image_line = Line::from(vec![
                        Span::raw("   "),
                        Span::styled(
                            truncate_string(
                                &container.image,
                                area.width.saturating_sub(7) as usize,
                            ),
                            Style::default().fg(styles::DIM_TEXT),
                        ),
                    ]);

                    ListItem::new(vec![line, image_line])
                }
            }
        })
        .collect();

    frame.render_widget(List::new(items).block(block), area);
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len > 3 {
        format!("{}...", &s[..max_len - 3])
    } else {
        s[..max_len].to_string()
    }
}
