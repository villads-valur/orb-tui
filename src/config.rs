use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

use crate::input::Keymap;
use crate::input::keybindings::{KeyBinding, action_names, parse_action, parse_key};

const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct Config {
    pub refresh_interval: Duration,
    pub keymap: Keymap,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            refresh_interval: DEFAULT_REFRESH_INTERVAL,
            keymap: Keymap::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    refresh_interval_secs: Option<u64>,
    #[serde(default)]
    keybindings: HashMap<String, OneOrMany>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

impl OneOrMany {
    fn into_vec(self) -> Vec<String> {
        match self {
            OneOrMany::One(key) => vec![key],
            OneOrMany::Many(keys) => keys,
        }
    }
}

/// `$XDG_CONFIG_HOME/orb-tui/config.toml`, falling back to `~/.config/orb-tui/config.toml`.
pub fn config_path() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => dirs::home_dir()?.join(".config"),
    };
    Some(base.join("orb-tui").join("config.toml"))
}

/// Load the config file. A missing file gives defaults silently. Any other
/// problem gives defaults plus an error message for the user.
pub fn load() -> (Config, Option<String>) {
    let Some(path) = config_path() else {
        return (Config::default(), None);
    };

    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return (Config::default(), None),
        Err(e) => {
            return (
                Config::default(),
                Some(error_message(&path, &e.to_string())),
            );
        }
    };

    match parse(&contents) {
        Ok(config) => (config, None),
        Err(reason) => (Config::default(), Some(error_message(&path, &reason))),
    }
}

fn error_message(path: &Path, reason: &str) -> String {
    format!(
        "Config error in {}: {reason}. Using defaults.",
        display_path(path)
    )
}

fn display_path(path: &Path) -> String {
    if let Some(home) = dirs::home_dir()
        && let Ok(relative) = path.strip_prefix(&home)
    {
        return format!("~/{}", relative.display());
    }
    path.display().to_string()
}

fn parse(contents: &str) -> Result<Config, String> {
    let raw: RawConfig = toml::from_str(contents).map_err(|e| e.message().to_string())?;

    let refresh_interval = match raw.refresh_interval_secs {
        Some(0) => return Err("refresh_interval_secs must be at least 1".to_string()),
        Some(secs) => Duration::from_secs(secs),
        None => DEFAULT_REFRESH_INTERVAL,
    };

    let mut names: Vec<_> = raw.keybindings.into_iter().collect();
    // Sort so duplicate-key errors are deterministic
    names.sort_by(|a, b| a.0.cmp(&b.0));

    let mut key_owner: HashMap<KeyBinding, String> = HashMap::new();
    let mut overrides = Vec::new();

    for (name, keys) in names {
        let Some(action) = parse_action(&name) else {
            let valid = action_names().collect::<Vec<_>>().join(", ");
            return Err(format!("unknown action '{name}' (valid actions: {valid})"));
        };

        let mut bindings = Vec::new();
        for key_str in keys.into_vec() {
            let binding = parse_key(&key_str).map_err(|e| format!("{e} for action '{name}'"))?;
            if let Some(owner) = key_owner.get(&binding)
                && owner != &name
            {
                return Err(format!(
                    "key '{key_str}' is bound to both '{owner}' and '{name}'"
                ));
            }
            key_owner.insert(binding, name.clone());
            bindings.push(binding);
        }
        overrides.push((action, bindings));
    }

    Ok(Config {
        refresh_interval,
        keymap: Keymap::with_overrides(overrides),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::InputMode;
    use crate::input::{Action, map_key_to_action};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn action_for(config: &Config, code: KeyCode) -> Action {
        map_key_to_action(
            KeyEvent::new(code, KeyModifiers::NONE),
            InputMode::Normal,
            &config.keymap,
        )
    }

    #[test]
    fn empty_config_uses_defaults() {
        let config = parse("").unwrap();
        assert_eq!(config.refresh_interval, DEFAULT_REFRESH_INTERVAL);
        assert_eq!(action_for(&config, KeyCode::Char('q')), Action::Quit);
    }

    #[test]
    fn parses_refresh_interval() {
        let config = parse("refresh_interval_secs = 2").unwrap();
        assert_eq!(config.refresh_interval, Duration::from_secs(2));
    }

    #[test]
    fn rejects_zero_refresh_interval() {
        assert!(parse("refresh_interval_secs = 0").is_err());
    }

    #[test]
    fn accepts_string_or_array_bindings() {
        let config = parse(
            r#"
            [keybindings]
            quit = "Q"
            move_down = ["n", "down"]
            "#,
        )
        .unwrap();
        assert_eq!(action_for(&config, KeyCode::Char('Q')), Action::Quit);
        assert_eq!(action_for(&config, KeyCode::Char('q')), Action::Noop);
        assert_eq!(action_for(&config, KeyCode::Char('n')), Action::MoveDown);
        assert_eq!(action_for(&config, KeyCode::Down), Action::MoveDown);
        assert_eq!(action_for(&config, KeyCode::Char('j')), Action::Noop);
    }

    #[test]
    fn rejects_unknown_action() {
        let err = parse("[keybindings]\nexplode = \"e\"").unwrap_err();
        assert!(err.contains("unknown action 'explode'"), "{err}");
    }

    #[test]
    fn rejects_invalid_key() {
        let err = parse("[keybindings]\nquit = \"ctrl+\"").unwrap_err();
        assert!(err.contains("invalid key 'ctrl+'"), "{err}");
    }

    #[test]
    fn rejects_unknown_field() {
        assert!(parse("refresh_interval = 5").is_err());
    }

    #[test]
    fn rejects_key_bound_to_two_actions() {
        let err = parse("[keybindings]\nquit = \"x\"\nstop_container = \"x\"").unwrap_err();
        assert!(
            err.contains("bound to both 'quit' and 'stop_container'"),
            "{err}"
        );
    }
}
