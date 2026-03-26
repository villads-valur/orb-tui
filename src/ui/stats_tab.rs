use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::ui::styles;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let container = match app.selected_container() {
        Some(c) => c,
        None => {
            render_placeholder(frame, area, "No Selection");
            return;
        }
    };

    let stats = match app.container_stats.get(&container.id) {
        Some(s) => s,
        None => {
            let msg = if app.detail_dirty {
                "Loading..."
            } else {
                "No stats available (container may not be running)"
            };
            render_placeholder(frame, area, msg);
            return;
        }
    };

    let detail = app.info_cache.get(&container.id);

    let mut lines = vec![
        Line::from(Span::styled("Resource Usage", styles::header_style())),
        Line::from(""),
    ];

    // CPU with limit info
    let cpu_display = match detail.and_then(|d| d.cpu_limit) {
        Some(limit) => format!("{} (limit: {} CPUs)", stats.cpu_perc, format_cpu(limit)),
        None => format!("{} (all CPUs)", stats.cpu_perc),
    };
    lines.push(make_stat_line("CPU", &cpu_display));

    // Memory with limit info
    let mem_display = match detail.and_then(|d| d.memory_limit) {
        Some(limit) => format!(
            "{} ({}) [limit: {}]",
            stats.mem_usage,
            stats.mem_perc,
            format_bytes(limit)
        ),
        None => format!("{} ({})", stats.mem_usage, stats.mem_perc),
    };
    lines.push(make_stat_line("Memory", &mem_display));

    lines.push(make_stat_line("Network I/O", &stats.net_io));
    lines.push(make_stat_line("Block I/O", &stats.block_io));
    lines.push(make_stat_line("PIDs", &stats.pids));

    frame.render_widget(Paragraph::new(lines), area);
}

pub fn render_group(frame: &mut Frame, app: &App, area: Rect) {
    let Some(project) = app.selected_group_project() else {
        return;
    };

    let containers = app.group_containers(project);
    if containers.is_empty() {
        render_placeholder(frame, area, "No containers in group");
        return;
    }

    let mut lines = vec![
        Line::from(Span::styled(
            format!("Resource Usage ({project})"),
            styles::header_style(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("{:<24}", "Service"), styles::label_style()),
            Span::styled(format!("{:<10}", "CPU"), styles::label_style()),
            Span::styled(format!("{:<24}", "Memory"), styles::label_style()),
            Span::styled("Net I/O", styles::label_style()),
        ]),
    ];

    let mut total_pids: u64 = 0;

    for container in &containers {
        let name = container.display_name();
        let state_color = styles::container_state_color(&container.state);

        if let Some(stats) = app.container_stats.get(&container.id) {
            if let Ok(pids) = stats.pids.trim().parse::<u64>() {
                total_pids += pids;
            }

            lines.push(Line::from(vec![
                Span::styled(
                    format!("{} {:<22}", container.state.symbol(), name),
                    Style::default().fg(state_color),
                ),
                Span::styled(format!("{:<10}", stats.cpu_perc), styles::value_style()),
                Span::styled(format!("{:<24}", stats.mem_usage), styles::value_style()),
                Span::styled(&stats.net_io, styles::value_style()),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{} {:<22}", container.state.symbol(), name),
                    Style::default().fg(state_color),
                ),
                Span::styled(format!("{:<10}", "-"), styles::label_style()),
                Span::styled(format!("{:<24}", "-"), styles::label_style()),
                Span::styled("-", styles::label_style()),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(format!("{:<24}", "Total PIDs"), styles::label_style()),
        Span::styled(total_pids.to_string(), styles::value_style()),
    ]));

    let visible_lines: Vec<Line> = lines.into_iter().skip(app.detail_scroll_offset).collect();
    frame.render_widget(Paragraph::new(visible_lines), area);
}

fn render_placeholder(frame: &mut Frame, area: Rect, msg: &str) {
    let paragraph = Paragraph::new(msg)
        .style(Style::default().fg(styles::DIM_TEXT))
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(paragraph, area);
}

fn make_stat_line<'a>(label: &'a str, value: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{label:<16}"), styles::label_style()),
        Span::styled(value, styles::value_style()),
    ])
}

fn format_cpu(cpus: f64) -> String {
    if cpus == cpus.floor() {
        format!("{}", cpus as u64)
    } else {
        format!("{cpus:.1}")
    }
}

fn format_bytes(bytes: u64) -> String {
    const GIB: u64 = 1024 * 1024 * 1024;
    const MIB: u64 = 1024 * 1024;

    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else {
        format!("{:.0} MiB", bytes as f64 / MIB as f64)
    }
}
