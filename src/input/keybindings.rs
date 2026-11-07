use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::InputMode;
use crate::input::action::Action;

pub type KeyBinding = (KeyCode, KeyModifiers);

/// Normal-mode actions that can be remapped from the config file, keyed by config name.
const REMAPPABLE_ACTIONS: &[(&str, Action)] = &[
    ("quit", Action::Quit),
    ("move_up", Action::MoveUp),
    ("move_down", Action::MoveDown),
    ("select", Action::Select),
    ("back", Action::Back),
    ("next_tab", Action::NextTab),
    ("prev_tab", Action::PrevTab),
    ("tab_info", Action::TabInfo),
    ("tab_logs", Action::TabLogs),
    ("tab_stats", Action::TabStats),
    ("start_container", Action::StartContainer),
    ("stop_container", Action::StopContainer),
    ("restart_container", Action::RestartContainer),
    ("remove_container", Action::RemoveContainer),
    ("toggle_log_follow", Action::ToggleLogFollow),
    ("toggle_full_screen", Action::ToggleFullScreen),
    ("scroll_half_page_up", Action::ScrollHalfPageUp),
    ("scroll_half_page_down", Action::ScrollHalfPageDown),
    ("scroll_to_top", Action::ScrollToTop),
    ("scroll_to_bottom", Action::ScrollToBottom),
    ("enter_search", Action::EnterSearch),
    ("clear_filter", Action::ClearFilter),
    ("open_orbstack", Action::OpenOrbStack),
];

fn default_bindings() -> Vec<(KeyBinding, Action)> {
    let none = KeyModifiers::NONE;
    let ctrl = KeyModifiers::CONTROL;
    vec![
        ((KeyCode::Char('q'), none), Action::Quit),
        ((KeyCode::Esc, none), Action::ClearFilter),
        ((KeyCode::Char('j'), none), Action::MoveDown),
        ((KeyCode::Down, none), Action::MoveDown),
        ((KeyCode::Char('k'), none), Action::MoveUp),
        ((KeyCode::Up, none), Action::MoveUp),
        ((KeyCode::Enter, none), Action::Select),
        ((KeyCode::Char('l'), none), Action::Select),
        ((KeyCode::Right, none), Action::Select),
        ((KeyCode::Char('h'), none), Action::Back),
        ((KeyCode::Left, none), Action::Back),
        ((KeyCode::Tab, none), Action::NextTab),
        ((KeyCode::BackTab, none), Action::PrevTab),
        ((KeyCode::Char('1'), none), Action::TabInfo),
        ((KeyCode::Char('2'), none), Action::TabLogs),
        ((KeyCode::Char('3'), none), Action::TabStats),
        ((KeyCode::Char('s'), none), Action::StartContainer),
        ((KeyCode::Char('x'), none), Action::StopContainer),
        ((KeyCode::Char('r'), none), Action::RestartContainer),
        ((KeyCode::Char('d'), none), Action::RemoveContainer),
        ((KeyCode::Char('f'), none), Action::ToggleLogFollow),
        ((KeyCode::Char('z'), none), Action::ToggleFullScreen),
        ((KeyCode::Char('/'), none), Action::EnterSearch),
        ((KeyCode::Char('o'), none), Action::OpenOrbStack),
        ((KeyCode::Char('g'), none), Action::ScrollToTop),
        ((KeyCode::Char('G'), none), Action::ScrollToBottom),
        ((KeyCode::Char('u'), ctrl), Action::ScrollHalfPageUp),
        ((KeyCode::Char('d'), ctrl), Action::ScrollHalfPageDown),
    ]
}

/// Normal-mode key to action table. Starts from the defaults and can be
/// overridden per action from the config file.
#[derive(Debug, Clone)]
pub struct Keymap {
    bindings: HashMap<KeyBinding, Action>,
}

impl Default for Keymap {
    fn default() -> Self {
        Keymap {
            bindings: default_bindings().into_iter().collect(),
        }
    }
}

impl Keymap {
    /// Each overridden action loses its default keys. A key bound by the user
    /// is also taken away from whichever action had it by default.
    pub fn with_overrides(overrides: Vec<(Action, Vec<KeyBinding>)>) -> Self {
        let mut keymap = Keymap::default();
        for (action, _) in &overrides {
            keymap.bindings.retain(|_, bound| bound != action);
        }
        for (action, keys) in overrides {
            for key in keys {
                keymap.bindings.insert(key, action.clone());
            }
        }
        keymap
    }

    fn lookup(&self, key: KeyEvent) -> Action {
        let binding = normalize(key);
        if binding == (KeyCode::Char('c'), KeyModifiers::CONTROL) {
            return Action::Quit;
        }
        self.bindings.get(&binding).cloned().unwrap_or(Action::Noop)
    }
}

/// Keep only Ctrl/Alt. Shift is already encoded in the char (`G`) or key code (`BackTab`).
fn normalize(key: KeyEvent) -> KeyBinding {
    let modifiers = key.modifiers & (KeyModifiers::CONTROL | KeyModifiers::ALT);
    (key.code, modifiers)
}

pub fn parse_action(name: &str) -> Option<Action> {
    REMAPPABLE_ACTIONS
        .iter()
        .find(|(action_name, _)| *action_name == name)
        .map(|(_, action)| action.clone())
}

pub fn action_names() -> impl Iterator<Item = &'static str> {
    REMAPPABLE_ACTIONS.iter().map(|(name, _)| *name)
}

/// Parse a key string such as `j`, `G`, `enter`, `shift+tab` or `ctrl+u`.
pub fn parse_key(input: &str) -> Result<KeyBinding, String> {
    let mut modifiers = KeyModifiers::NONE;
    let mut rest = input;

    loop {
        let lower = rest.to_ascii_lowercase();
        if let Some(stripped) = lower.strip_prefix("ctrl+") {
            modifiers |= KeyModifiers::CONTROL;
            rest = &rest[rest.len() - stripped.len()..];
        } else if let Some(stripped) = lower.strip_prefix("alt+") {
            modifiers |= KeyModifiers::ALT;
            rest = &rest[rest.len() - stripped.len()..];
        } else {
            break;
        }
    }

    let invalid = || format!("invalid key '{input}'");

    let mut chars = rest.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return Ok((KeyCode::Char(c), modifiers));
    }

    let code = match rest.to_ascii_lowercase().as_str() {
        "enter" | "return" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "tab" => KeyCode::Tab,
        "backtab" | "shift+tab" => KeyCode::BackTab,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "backspace" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "space" => KeyCode::Char(' '),
        _ => return Err(invalid()),
    };
    Ok((code, modifiers))
}

pub fn map_key_to_action(key: KeyEvent, mode: InputMode, keymap: &Keymap) -> Action {
    match mode {
        InputMode::Confirm => map_confirm_key(key),
        InputMode::Normal => keymap.lookup(key),
        InputMode::Search => map_search_key(key),
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

    fn press(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    fn normal(keymap: &Keymap, code: KeyCode, modifiers: KeyModifiers) -> Action {
        map_key_to_action(press(code, modifiers), InputMode::Normal, keymap)
    }

    #[test]
    fn parses_single_chars() {
        assert_eq!(parse_key("j"), Ok((KeyCode::Char('j'), KeyModifiers::NONE)));
        assert_eq!(parse_key("G"), Ok((KeyCode::Char('G'), KeyModifiers::NONE)));
        assert_eq!(parse_key("/"), Ok((KeyCode::Char('/'), KeyModifiers::NONE)));
    }

    #[test]
    fn parses_named_keys_case_insensitively() {
        assert_eq!(parse_key("Enter"), Ok((KeyCode::Enter, KeyModifiers::NONE)));
        assert_eq!(parse_key("DOWN"), Ok((KeyCode::Down, KeyModifiers::NONE)));
        assert_eq!(
            parse_key("space"),
            Ok((KeyCode::Char(' '), KeyModifiers::NONE))
        );
        assert_eq!(
            parse_key("shift+tab"),
            Ok((KeyCode::BackTab, KeyModifiers::NONE))
        );
    }

    #[test]
    fn parses_modifiers() {
        assert_eq!(
            parse_key("ctrl+u"),
            Ok((KeyCode::Char('u'), KeyModifiers::CONTROL))
        );
        assert_eq!(
            parse_key("Ctrl+Alt+x"),
            Ok((
                KeyCode::Char('x'),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            ))
        );
        assert_eq!(
            parse_key("alt+enter"),
            Ok((KeyCode::Enter, KeyModifiers::ALT))
        );
    }

    #[test]
    fn rejects_invalid_keys() {
        assert!(parse_key("").is_err());
        assert!(parse_key("ctrl+").is_err());
        assert!(parse_key("banana").is_err());
    }

    #[test]
    fn default_keymap_matches_builtin_keys() {
        let keymap = Keymap::default();
        assert_eq!(
            normal(&keymap, KeyCode::Char('j'), KeyModifiers::NONE),
            Action::MoveDown
        );
        assert_eq!(
            normal(&keymap, KeyCode::Char('d'), KeyModifiers::NONE),
            Action::RemoveContainer
        );
        assert_eq!(
            normal(&keymap, KeyCode::Char('d'), KeyModifiers::CONTROL),
            Action::ScrollHalfPageDown
        );
        assert_eq!(
            normal(&keymap, KeyCode::Char('G'), KeyModifiers::SHIFT),
            Action::ScrollToBottom
        );
        assert_eq!(
            normal(&keymap, KeyCode::BackTab, KeyModifiers::SHIFT),
            Action::PrevTab
        );
        assert_eq!(
            normal(&keymap, KeyCode::Char('w'), KeyModifiers::NONE),
            Action::Noop
        );
    }

    #[test]
    fn override_replaces_default_keys_for_that_action() {
        let keymap = Keymap::with_overrides(vec![(
            Action::Quit,
            vec![(KeyCode::Char('Q'), KeyModifiers::NONE)],
        )]);
        assert_eq!(
            normal(&keymap, KeyCode::Char('Q'), KeyModifiers::SHIFT),
            Action::Quit
        );
        assert_eq!(
            normal(&keymap, KeyCode::Char('q'), KeyModifiers::NONE),
            Action::Noop
        );
        assert_eq!(
            normal(&keymap, KeyCode::Char('j'), KeyModifiers::NONE),
            Action::MoveDown
        );
    }

    #[test]
    fn override_takes_key_from_other_action() {
        let keymap = Keymap::with_overrides(vec![(
            Action::Quit,
            vec![(KeyCode::Char('x'), KeyModifiers::NONE)],
        )]);
        assert_eq!(
            normal(&keymap, KeyCode::Char('x'), KeyModifiers::NONE),
            Action::Quit
        );
    }

    #[test]
    fn ctrl_c_always_quits() {
        let keymap = Keymap::with_overrides(vec![(
            Action::StopContainer,
            vec![(KeyCode::Char('c'), KeyModifiers::CONTROL)],
        )]);
        assert_eq!(
            normal(&keymap, KeyCode::Char('c'), KeyModifiers::CONTROL),
            Action::Quit
        );
    }

    #[test]
    fn search_and_confirm_modes_ignore_keymap() {
        let keymap = Keymap::default();
        assert_eq!(
            map_key_to_action(
                press(KeyCode::Char('q'), KeyModifiers::NONE),
                InputMode::Search,
                &keymap
            ),
            Action::SearchType('q')
        );
        assert_eq!(
            map_key_to_action(
                press(KeyCode::Char('y'), KeyModifiers::NONE),
                InputMode::Confirm,
                &keymap
            ),
            Action::ConfirmYes
        );
    }
}
