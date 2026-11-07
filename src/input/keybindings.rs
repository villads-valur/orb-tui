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
        KeyCode::Char('z') => Action::ToggleFullScreen,

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

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn normal(event: KeyEvent) -> Action {
        map_key_to_action(event, InputMode::Normal)
    }

    #[test]
    fn normal_mode_maps_core_keys() {
        let cases = [
            (KeyCode::Char('q'), Action::Quit),
            (KeyCode::Esc, Action::ClearFilter),
            (KeyCode::Char('j'), Action::MoveDown),
            (KeyCode::Down, Action::MoveDown),
            (KeyCode::Char('k'), Action::MoveUp),
            (KeyCode::Up, Action::MoveUp),
            (KeyCode::Enter, Action::Select),
            (KeyCode::Char('l'), Action::Select),
            (KeyCode::Right, Action::Select),
            (KeyCode::Char('h'), Action::Back),
            (KeyCode::Left, Action::Back),
            (KeyCode::Tab, Action::NextTab),
            (KeyCode::BackTab, Action::PrevTab),
            (KeyCode::Char('1'), Action::TabInfo),
            (KeyCode::Char('2'), Action::TabLogs),
            (KeyCode::Char('3'), Action::TabStats),
            (KeyCode::Char('s'), Action::StartContainer),
            (KeyCode::Char('x'), Action::StopContainer),
            (KeyCode::Char('r'), Action::RestartContainer),
            (KeyCode::Char('d'), Action::RemoveContainer),
            (KeyCode::Char('f'), Action::ToggleLogFollow),
            (KeyCode::Char('z'), Action::ToggleFullScreen),
            (KeyCode::Char('/'), Action::EnterSearch),
            (KeyCode::Char('o'), Action::OpenOrbStack),
            (KeyCode::Char('g'), Action::ScrollToTop),
            (KeyCode::Char('G'), Action::ScrollToBottom),
        ];
        for (code, expected) in cases {
            assert_eq!(normal(key(code)), expected, "key: {code:?}");
        }
    }

    #[test]
    fn normal_mode_ctrl_keys() {
        assert_eq!(normal(ctrl(KeyCode::Char('c'))), Action::Quit);
        assert_eq!(normal(ctrl(KeyCode::Char('u'))), Action::ScrollHalfPageUp);
        assert_eq!(normal(ctrl(KeyCode::Char('d'))), Action::ScrollHalfPageDown);
    }

    #[test]
    fn ctrl_overrides_plain_binding() {
        assert_ne!(normal(ctrl(KeyCode::Char('d'))), Action::RemoveContainer);
        assert_eq!(normal(ctrl(KeyCode::Char('q'))), Action::Noop);
        assert_eq!(normal(ctrl(KeyCode::Char('j'))), Action::Noop);
    }

    #[test]
    fn normal_mode_unknown_key_is_noop() {
        assert_eq!(normal(key(KeyCode::Char('p'))), Action::Noop);
        assert_eq!(normal(key(KeyCode::F(1))), Action::Noop);
    }

    #[test]
    fn search_mode_maps_enter_esc_backspace() {
        let search = |code| map_key_to_action(key(code), InputMode::Search);
        assert_eq!(search(KeyCode::Enter), Action::SearchConfirm);
        assert_eq!(search(KeyCode::Esc), Action::SearchCancel);
        assert_eq!(search(KeyCode::Backspace), Action::SearchBackspace);
        assert_eq!(search(KeyCode::Up), Action::Noop);
    }

    #[test]
    fn search_mode_types_any_char_including_bound_ones() {
        for c in ['q', 'j', '/', 'G', ' '] {
            assert_eq!(
                map_key_to_action(key(KeyCode::Char(c)), InputMode::Search),
                Action::SearchType(c)
            );
        }
    }

    #[test]
    fn confirm_mode_yes_no_keys() {
        let confirm = |code| map_key_to_action(key(code), InputMode::Confirm);
        assert_eq!(confirm(KeyCode::Char('y')), Action::ConfirmYes);
        assert_eq!(confirm(KeyCode::Enter), Action::ConfirmYes);
        assert_eq!(confirm(KeyCode::Char('n')), Action::ConfirmNo);
        assert_eq!(confirm(KeyCode::Esc), Action::ConfirmNo);
    }

    #[test]
    fn confirm_mode_other_key_is_noop() {
        let confirm = |code| map_key_to_action(key(code), InputMode::Confirm);
        assert_eq!(confirm(KeyCode::Char('q')), Action::Noop);
        assert_eq!(confirm(KeyCode::Char('Y')), Action::Noop);
    }
}
