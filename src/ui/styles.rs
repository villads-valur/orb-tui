use ratatui::style::{Color, Modifier, Style};

use crate::docker::types::ContainerState;

pub const ACCENT: Color = Color::Cyan;
pub const SUBTLE: Color = Color::Gray;
pub const TEXT: Color = Color::Reset;
pub const DIM_TEXT: Color = Color::Gray;

pub fn container_state_color(state: &ContainerState) -> Color {
    match state {
        ContainerState::Running => Color::Green,
        ContainerState::Exited => Color::Red,
        ContainerState::Paused => Color::Yellow,
        ContainerState::Restarting => Color::Yellow,
        ContainerState::Dead => Color::Red,
        ContainerState::Created => Color::Cyan,
        ContainerState::Removing => Color::DarkGray,
    }
}

pub fn selected_style() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub fn tab_active_style() -> Style {
    Style::default()
        .fg(ACCENT)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
}

pub fn tab_inactive_style() -> Style {
    Style::default().fg(DIM_TEXT)
}

pub fn header_style() -> Style {
    Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

pub fn label_style() -> Style {
    Style::default().fg(DIM_TEXT)
}

pub fn value_style() -> Style {
    Style::default().fg(TEXT)
}

pub fn group_header_style() -> Style {
    Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

pub fn border_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(ACCENT)
    } else {
        Style::default().fg(SUBTLE)
    }
}

pub fn statusbar_style() -> Style {
    Style::default().fg(DIM_TEXT)
}
