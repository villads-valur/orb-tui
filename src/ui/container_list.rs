use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

use crate::app::{App, FocusedPanel, ListItem as AppListItem};
use crate::ui::styles;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focused_panel == FocusedPanel::ContainerList;
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
                    let line = Line::from(vec![
                        Span::styled(format!("{arrow} "), Style::default().fg(styles::DIM_TEXT)),
                        Span::styled(project.as_str(), styles::group_header_style()),
                        Span::styled(count_str, Style::default().fg(styles::DIM_TEXT)),
                    ]);

                    if is_selected {
                        ListItem::new(line).style(styles::selected_style())
                    } else {
                        ListItem::new(line)
                    }
                }
                AppListItem::Container { index } => {
                    let container = &app.containers[*index];
                    let state_color = styles::container_state_color(&container.state);
                    let symbol = container.state.symbol();
                    let display_name = container.display_name();

                    let name_style = if is_selected {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(styles::TEXT)
                    };

                    let image_fg = if is_selected {
                        Color::Gray
                    } else {
                        styles::DIM_TEXT
                    };

                    let line = Line::from(vec![
                        Span::raw("  "),
                        Span::styled(format!("{symbol} "), Style::default().fg(state_color)),
                        Span::styled(display_name, name_style),
                    ]);

                    let image_line = Line::from(vec![
                        Span::raw("    "),
                        Span::styled(
                            truncate_string(
                                &container.image,
                                area.width.saturating_sub(8) as usize,
                            ),
                            Style::default().fg(image_fg),
                        ),
                    ]);

                    let style = if is_selected {
                        styles::selected_style()
                    } else {
                        Style::default()
                    };

                    ListItem::new(vec![line, image_line]).style(style)
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
