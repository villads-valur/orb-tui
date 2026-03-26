use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::ui::styles;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let Some(detail) = app.selected_detail() else {
        let msg = if app.detail_dirty {
            "Loading..."
        } else {
            "No Selection"
        };
        let paragraph = Paragraph::new(msg)
            .style(Style::default().fg(styles::DIM_TEXT))
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(paragraph, area);
        return;
    };

    let mut lines: Vec<Line> = Vec::new();

    let fields = [
        ("Name", detail.name.as_str()),
        ("ID", detail.id.as_str()),
        ("Image", detail.image.as_str()),
        ("Status", detail.status.as_str()),
    ];

    for (label, value) in &fields {
        lines.push(Line::from(vec![
            Span::styled(format!("{label:<16}"), styles::label_style()),
            Span::styled(*value, styles::value_style()),
        ]));
    }

    if let Some(domain) = &detail.domain {
        lines.push(Line::from(vec![
            Span::styled(format!("{:<16}", "Domain"), styles::label_style()),
            Span::styled(domain.as_str(), Style::default().fg(styles::ACCENT)),
        ]));
    }

    if let Some(ip) = &detail.ip_address {
        lines.push(Line::from(vec![
            Span::styled(format!("{:<16}", "IP"), styles::label_style()),
            Span::styled(ip.as_str(), styles::value_style()),
        ]));
    }

    lines.push(Line::from(vec![
        Span::styled(format!("{:<16}", "Network"), styles::label_style()),
        Span::styled(detail.network_mode.as_str(), styles::value_style()),
    ]));

    lines.push(Line::from(vec![
        Span::styled(format!("{:<16}", "Restart"), styles::label_style()),
        Span::styled(detail.restart_policy.as_str(), styles::value_style()),
    ]));

    if !detail.port_forwards.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Port Forwards",
            styles::header_style(),
        )));

        lines.push(Line::from(vec![
            Span::styled(format!("{:<16}", "Host Port"), styles::label_style()),
            Span::styled(format!("{:<18}", "Container Port"), styles::label_style()),
            Span::styled("Protocol", styles::label_style()),
        ]));

        for pf in &detail.port_forwards {
            lines.push(Line::from(vec![
                Span::styled(format!("{:<16}", pf.host_port), styles::value_style()),
                Span::styled(format!("{:<18}", pf.container_port), styles::value_style()),
                Span::styled(pf.protocol.as_str(), styles::value_style()),
            ]));
        }
    }

    if !detail.labels.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Labels", styles::header_style())));

        let mut sorted_labels: Vec<_> = detail.labels.iter().collect();
        sorted_labels.sort_by_key(|(k, _)| k.as_str());

        let max_val_len = area.width.saturating_sub(40) as usize;
        for (key, value) in sorted_labels {
            lines.push(Line::from(vec![
                Span::styled(format!("{:<36}", truncate(key, 35)), styles::label_style()),
                Span::styled(truncate(value, max_val_len), styles::value_style()),
            ]));
        }
    }

    let visible_lines: Vec<Line> = lines.into_iter().skip(app.detail_scroll_offset).collect();
    frame.render_widget(Paragraph::new(visible_lines), area);
}

pub fn render_group(frame: &mut Frame, app: &App, area: Rect) {
    let Some(project) = app.selected_group_project() else {
        return;
    };

    let containers = app.group_containers(project);
    let running = containers.iter().filter(|c| c.state.is_running()).count();
    let total = containers.len();

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled(format!("{:<16}", "Project"), styles::label_style()),
            Span::styled(project, styles::header_style()),
        ]),
        Line::from(vec![
            Span::styled(format!("{:<16}", "Containers"), styles::label_style()),
            Span::styled(
                format!("{running} running / {total} total"),
                styles::value_style(),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("Services", styles::header_style())),
    ];

    for container in &containers {
        let state_color = styles::container_state_color(&container.state);
        let symbol = container.state.symbol();
        let name = container.display_name();
        let image = truncate(&container.image, area.width.saturating_sub(30) as usize);

        lines.push(Line::from(vec![
            Span::styled(format!("{symbol} "), Style::default().fg(state_color)),
            Span::styled(format!("{name:<24}"), styles::value_style()),
            Span::styled(image, styles::label_style()),
        ]));
    }

    let visible_lines: Vec<Line> = lines.into_iter().skip(app.detail_scroll_offset).collect();
    frame.render_widget(Paragraph::new(visible_lines), area);
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else if max > 3 {
        format!("{}...", &s[..max - 3])
    } else {
        s[..max].to_string()
    }
}
