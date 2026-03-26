pub mod container_list;
pub mod detail_panel;
pub mod info_tab;
pub mod logs_tab;
pub mod stats_tab;
pub mod status_bar;
pub mod styles;

use ratatui::layout::{Constraint, Layout};
use ratatui::Frame;

use crate::app::App;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let vertical = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);

    let horizontal = Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(vertical[1]);

    status_bar::render_header(frame, app, vertical[0]);
    container_list::render(frame, app, horizontal[0]);
    detail_panel::render(frame, app, horizontal[1]);
    status_bar::render_footer(frame, app, vertical[2]);
}
