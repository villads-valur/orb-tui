use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, DetailTab, FocusedPanel};
use crate::ui::{info_tab, logs_tab, stats_tab, styles};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focused_panel == FocusedPanel::Detail;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(styles::border_style(focused));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 3 || inner.width < 10 {
        return;
    }

    // Split inner area: tab bar (1 line) + content
    let chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(inner);

    // Render tab bar
    render_tab_bar(frame, app, chunks[0]);

    // Add padding for content
    let content_area = Rect {
        x: chunks[1].x + 1,
        y: chunks[1].y + 1,
        width: chunks[1].width.saturating_sub(2),
        height: chunks[1].height.saturating_sub(1),
    };

    if content_area.width == 0 || content_area.height == 0 {
        return;
    }

    let is_group = app.selected_group_project().is_some();
    let is_container = app.selected_container().is_some();

    if !is_group && !is_container {
        let paragraph = Paragraph::new("No Selection")
            .style(Style::default().fg(styles::DIM_TEXT))
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(paragraph, content_area);
        return;
    }

    match app.detail_tab {
        DetailTab::Info => {
            if is_group {
                info_tab::render_group(frame, app, content_area);
            } else {
                info_tab::render(frame, app, content_area);
            }
        }
        DetailTab::Logs => logs_tab::render(frame, app, content_area),
        DetailTab::Stats => {
            if is_group {
                stats_tab::render_group(frame, app, content_area);
            } else {
                stats_tab::render(frame, app, content_area);
            }
        }
    }
}

fn render_tab_bar(frame: &mut Frame, app: &App, area: Rect) {
    let tabs = [
        (DetailTab::Info, "1 Info"),
        (DetailTab::Logs, "2 Logs"),
        (DetailTab::Stats, "3 Stats"),
    ];

    let mut spans = Vec::new();
    for (i, (tab, label)) in tabs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" │ ", Style::default().fg(styles::SUBTLE)));
        }
        let style = if app.detail_tab == *tab {
            styles::tab_active_style()
        } else {
            styles::tab_inactive_style()
        };
        spans.push(Span::styled(*label, style));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
