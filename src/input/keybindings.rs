use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::InputMode;
use crate::input::action::Action;

pub fn map_key_to_action(key: KeyEvent, mode: InputMode) -> Action {
    match mode {
        InputMode::Confirm => map_confirm_key(key),
        InputMode::Normal => map_normal_key(key),
        InputMode::Search => map_search_key(key),
    }
}

fn map_normal_key(key: KeyEvent) -> Action {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') => Action::Quit,
            KeyCode::Char('u') => Action::ScrollHalfPageUp,
            KeyCode::Char('d') => Action::ScrollHalfPageDown,
            _ => Action::Noop,
        };
    }

    match key.code {
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Esc => Action::ClearFilter,

        KeyCode::Char('j') | KeyCode::Down => Action::MoveDown,
        KeyCode::Char('k') | KeyCode::Up => Action::MoveUp,
        KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => Action::Select,
        KeyCode::Char('h') | KeyCode::Left => Action::Back,

        KeyCode::Tab => Action::NextTab,
        KeyCode::BackTab => Action::PrevTab,
        KeyCode::Char('1') => Action::TabInfo,
        KeyCode::Char('2') => Action::TabLogs,
        KeyCode::Char('3') => Action::TabStats,

        KeyCode::Char('s') => Action::StartContainer,
        KeyCode::Char('x') => Action::StopContainer,
        KeyCode::Char('r') => Action::RestartContainer,
        KeyCode::Char('d') => Action::RemoveContainer,

        KeyCode::Char('f') => Action::ToggleLogFollow,

        KeyCode::Char('/') => Action::EnterSearch,
        KeyCode::Char('o') => Action::OpenOrbStack,

        KeyCode::Char('g') => Action::ScrollToTop,
        KeyCode::Char('G') => Action::ScrollToBottom,

        _ => Action::Noop,
    }
}

fn map_search_key(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => Action::SearchConfirm,
        KeyCode::Esc => Action::SearchCancel,
        KeyCode::Backspace => Action::SearchBackspace,
        KeyCode::Char(c) => Action::SearchType(c),
        _ => Action::Noop,
    }
}

fn map_confirm_key(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('y') | KeyCode::Enter => Action::ConfirmYes,
        KeyCode::Char('n') | KeyCode::Esc => Action::ConfirmNo,
        _ => Action::Noop,
    }
}
