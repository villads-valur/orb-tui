use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::ui::styles;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let Some(logs) = &app.container_logs else {
        let msg = if app.detail_dirty {
            "Loading..."
        } else {
            "No logs available"
        };
        let paragraph = Paragraph::new(msg)
            .style(Style::default().fg(styles::DIM_TEXT))
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(paragraph, area);
        return;
    };

    let log_lines: Vec<Line> = logs.lines().map(|l| Line::from(l.to_string())).collect();
    let total_lines = log_lines.len();
    let visible_height = area.height as usize;

    // Calculate scroll offset
    let scroll_offset = if app.log_follow {
        // Auto-scroll to bottom
        total_lines.saturating_sub(visible_height)
    } else {
        app.log_scroll_offset
            .min(total_lines.saturating_sub(visible_height))
    };

    let visible_lines: Vec<Line> = log_lines
        .into_iter()
        .skip(scroll_offset)
        .take(visible_height)
        .collect();

    let paragraph = Paragraph::new(visible_lines).style(Style::default().fg(styles::TEXT));

    frame.render_widget(paragraph, area);
}
