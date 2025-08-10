use std::{collections::HashMap, fmt::Debug, fs::write, path::PathBuf};

use bitflags::Flags;
use bpm_detection_core::parameters::{DynamicBPMDetectionConfig, MidiServiceConfig, StaticBPMDetectionConfig};
use build::{get_config_dir, get_data_dir};
use config::ConfigError;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use derive_deref::{Deref, DerefMut};
use errors::{Report, Result, TypedResult};
use gui::GUIConfig;
use itertools::Itertools;
use log::{error, info};
use ratatui::style::Style;
use serde::{
    Deserialize, Serialize, Serializer,
    de::{Deserializer, Error},
    ser,
    ser::SerializeMap,
};
use strum::Display;

use crate::{action::Action, mode::Mode};

const CONFIG: &str = include_str!("../config/base_config.toml");

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TUIConfig {
    #[serde(default, flatten)]
    #[allow(forbidden_lint_groups)]
    #[allow(clippy::struct_field_names)]
    #[serde(skip_serializing)]
    pub app_config: AppConfig,
    #[serde(default)]
    pub keybindings: KeyBindings,
    #[serde(default)]
    pub styles: HashMap<Mode, HashMap<String, Style>>,
    pub frame_rate: f64,
    pub tick_rate: f64,
    #[serde(rename = "GUI")]
    pub gui_config: GUIConfig,
    #[serde(rename = "MIDI")]
    pub midi: MidiServiceConfig,
    #[serde(default)]
    pub static_bpm_detection_config: StaticBPMDetectionConfig,
    #[serde(default)]
    pub dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
}

impl TUIConfig {
    pub fn new() -> TypedResult<Self, ConfigError> {
        let data_dir = get_data_dir();
        let config_dir = get_config_dir();
        let builder = config::Config::builder()
            .set_default("_data_dir", data_dir.to_str().unwrap())?
            .set_default("_config_dir", config_dir.to_str().unwrap())?
            .add_source(config::File::from_str(CONFIG, config::FileFormat::Toml))
            .add_source(
                config::File::from(config_dir.join("config.toml")).format(config::FileFormat::Toml).required(false),
            );

        Ok(builder.build()?.try_deserialize()?)
    }

    pub fn save(&self) -> Result<()> {
        let serialized = match toml::to_string_pretty(self) {
            Ok(serialized) => serialized,
            Err(e) => {
                error!("Serialization error: {e:?}");
                return Err(Report::new(e));
            }
        };

        let config_path = get_config_dir().join("config.toml");
        info!("configuration saved at {}", config_path.display());
        Ok(write(config_path, serialized)?)
    }
}

#[derive(Clone, Debug, Default, Deref, DerefMut)]
pub struct KeyBindings(pub HashMap<Option<Mode>, HashMap<Vec<KeyEvent>, Action>>);

impl Serialize for KeyBindings {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut main_map = serializer.serialize_map(Some(self.0.len()))?;

        for (mode, bindings) in &self.0 {
            // Log information about each key-value pair
            if let Some(mode) = mode.as_ref() {
                let value: HashMap<_, _> = bindings
                    .iter()
                    .map(|(key_events, action)| {
                        (format!("<{}>", key_events.iter().map(key_event_to_string).join("-")), action)
                    })
                    .collect();
                main_map.serialize_entry(&mode.to_string(), &value).map_err(|e| {
                    ser::Error::custom(Report::msg(format!("can't serialize mode entry {mode} {bindings:?} : {e:?}")))
                })?;
            } else {
                for (key_events, action) in bindings {
                    let key = format!("<{}>", key_events.iter().map(key_event_to_string).join("-"));
                    main_map.serialize_entry(&key, action).map_err(|e| {
                        ser::Error::custom(Report::msg(format!("can't serialize main entry {key}:{action:?} ({e:?})")))
                    })?;
                }
            }
        }

        main_map.end()
    }
}

#[derive(Deserialize, Eq, Hash, PartialEq, Display)]
#[serde(untagged)]
enum KeyOrMode {
    Mode(Mode),
    Key(String),
}

#[derive(Deserialize, Eq, PartialEq)]
#[serde(untagged)]
enum KeyMappingsOrAction {
    Keymapping(HashMap<String, Action>),
    Action(Action),
}

impl<'de> Deserialize<'de> for KeyBindings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let parsed_map = HashMap::<KeyOrMode, KeyMappingsOrAction>::deserialize(deserializer)?;

        let keybindings = parsed_map
            .into_iter()
            .try_fold(
                HashMap::<Option<Mode>, HashMap<Vec<KeyEvent>, Action>>::default(),
                |mut keybindings, (key_or_mode, keymapping_or_action)| {
                    match (key_or_mode, keymapping_or_action) {
                        (KeyOrMode::Mode(mode), KeyMappingsOrAction::Keymapping(mapping)) => {
                            keybindings.insert(
                                Some(mode),
                                mapping
                                    .into_iter()
                                    .map(|(key_str, cmd)| Ok((parse_key_sequence(&key_str)?, cmd)))
                                    .collect::<Result<_, String>>()?,
                            );
                        }
                        (KeyOrMode::Key(key), KeyMappingsOrAction::Action(action)) => {
                            let mappings = keybindings.entry(None).or_default();
                            let sequence = parse_key_sequence(key.as_str())?;
                            mappings.insert(sequence, action);
                        }
                        (KeyOrMode::Key(mode), KeyMappingsOrAction::Keymapping(_)) => {
                            return Err(format!("{mode} is not a valid mode"));
                        }
                        (KeyOrMode::Mode(mode), KeyMappingsOrAction::Action(action)) => {
                            return Err(format!("{mode} is a mode and cannot be assigned to an action ( {action} )"));
                        }
                    }

                    Ok(keybindings)
                },
            )
            .map_err(Error::custom)?;

        Ok(KeyBindings(keybindings))
    }
}

fn parse_key_event(raw: &str) -> Result<KeyEvent, String> {
    let raw_lower = raw.to_ascii_lowercase();
    let (remaining, modifiers) = extract_modifiers(&raw_lower);
    parse_key_code_with_modifiers(remaining, modifiers)
}

fn extract_modifiers(raw: &str) -> (&str, KeyModifiers) {
    let mut modifiers = KeyModifiers::empty();
    let mut current = raw;

    loop {
        match current {
            rest if rest.starts_with("ctrl-") => {
                modifiers.insert(KeyModifiers::CONTROL);
                current = &rest[5..];
            }
            rest if rest.starts_with("alt-") => {
                modifiers.insert(KeyModifiers::ALT);
                current = &rest[4..];
            }
            rest if rest.starts_with("shift-") => {
                modifiers.insert(KeyModifiers::SHIFT);
                current = &rest[6..];
            }
            _ => break, // break out of the loop if no known prefix is detected
        }
    }

    (current, modifiers)
}

fn parse_key_code_with_modifiers(raw: &str, mut modifiers: KeyModifiers) -> Result<KeyEvent, String> {
    let c = match raw {
        "esc" => KeyCode::Esc,
        "enter" => KeyCode::Enter,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "backtab" => {
            modifiers.insert(KeyModifiers::SHIFT);
            KeyCode::BackTab
        }
        "backspace" => KeyCode::Backspace,
        "delete" => KeyCode::Delete,
        "insert" => KeyCode::Insert,
        "f1" => KeyCode::F(1),
        "f2" => KeyCode::F(2),
        "f3" => KeyCode::F(3),
        "f4" => KeyCode::F(4),
        "f5" => KeyCode::F(5),
        "f6" => KeyCode::F(6),
        "f7" => KeyCode::F(7),
        "f8" => KeyCode::F(8),
        "f9" => KeyCode::F(9),
        "f10" => KeyCode::F(10),
        "f11" => KeyCode::F(11),
        "f12" => KeyCode::F(12),
        "space" => KeyCode::Char(' '),
        "hyphen" | "minus" => KeyCode::Char('-'),
        "tab" => KeyCode::Tab,
        c if c.len() == 1 => {
            let mut c = c.chars().next().unwrap();

            if Flags::contains(&modifiers, KeyModifiers::SHIFT) {
                c = c.to_ascii_uppercase();
            }

            KeyCode::Char(c)
        }
        _ => return Err(format!("Unable to parse {raw}")),
    };
    Ok(KeyEvent::new(c, modifiers))
}

#[must_use]
pub fn key_event_to_string(key_event: &KeyEvent) -> String {
    let char;
    let key_code = match key_event.code {
        KeyCode::Backspace => "backspace",
        KeyCode::Enter => "enter",
        KeyCode::Left => "left",
        KeyCode::Right => "right",
        KeyCode::Up => "up",
        KeyCode::Down => "down",
        KeyCode::Home => "home",
        KeyCode::End => "end",
        KeyCode::PageUp => "pageup",
        KeyCode::PageDown => "pagedown",
        KeyCode::Tab => "tab",
        KeyCode::BackTab => "backtab",
        KeyCode::Delete => "delete",
        KeyCode::Insert => "insert",
        KeyCode::F(c) => {
            char = format!("f({c})");
            &char
        }
        KeyCode::Char(' ') => "space",
        KeyCode::Char(c) => {
            char = c.to_string();
            &char
        }
        KeyCode::Esc => "esc",
        KeyCode::Null
        | KeyCode::ScrollLock
        | KeyCode::NumLock
        | KeyCode::CapsLock
        | KeyCode::Menu
        | KeyCode::Media(_)
        | KeyCode::PrintScreen
        | KeyCode::Pause
        | KeyCode::KeypadBegin
        | KeyCode::Modifier(_) => "",
    };

    let mut modifiers = Vec::with_capacity(3);

    if key_event.modifiers.intersects(KeyModifiers::CONTROL) {
        modifiers.push("ctrl");
    }

    if key_event.modifiers.intersects(KeyModifiers::SHIFT) {
        modifiers.push("shift");
    }

    if key_event.modifiers.intersects(KeyModifiers::ALT) {
        modifiers.push("alt");
    }

    let mut key = modifiers.join("-");

    if !key.is_empty() {
        key.push('-');
    }
    key.push_str(key_code);

    key
}

pub fn parse_key_sequence(raw: &str) -> Result<Vec<KeyEvent>, String> {
    if raw.chars().filter(|c| *c == '>').count() != raw.chars().filter(|c| *c == '<').count() {
        return Err(format!("Unable to parse `{raw}`"));
    }
    let raw = if raw.contains("><") {
        raw
    } else {
        let raw = raw.strip_prefix('<').unwrap_or(raw);
        raw.strip_prefix('>').unwrap_or(raw)
    };
    let sequences = raw
        .split("><")
        .map(|seq| {
            if let Some(s) = seq.strip_prefix('<') {
                s
            } else if let Some(s) = seq.strip_suffix('>') {
                s
            } else {
                seq
            }
        })
        .collect::<Vec<_>>();

    sequences.into_iter().map(parse_key_event).collect()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_config() -> Result<()> {
        let c = TUIConfig::new()?;

        assert_eq!(
            c.keybindings.get(&None).unwrap().get(&parse_key_sequence("<q>").unwrap_or_default()).unwrap(),
            &Action::Quit
        );
        Ok(())
    }

    #[test]
    fn test_simple_keys() {
        assert_eq!(parse_key_event("a").unwrap(), KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()));

        assert_eq!(parse_key_event("enter").unwrap(), KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));

        assert_eq!(parse_key_event("esc").unwrap(), KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    }

    #[test]
    fn test_with_modifiers() {
        assert_eq!(parse_key_event("ctrl-a").unwrap(), KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));

        assert_eq!(parse_key_event("alt-enter").unwrap(), KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT));

        assert_eq!(parse_key_event("shift-esc").unwrap(), KeyEvent::new(KeyCode::Esc, KeyModifiers::SHIFT));
    }

    #[test]
    fn test_multiple_modifiers() {
        assert_eq!(
            parse_key_event("ctrl-alt-a").unwrap(),
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL | KeyModifiers::ALT)
        );

        assert_eq!(
            parse_key_event("ctrl-shift-enter").unwrap(),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL | KeyModifiers::SHIFT)
        );
    }

    #[test]
    fn test_reverse_multiple_modifiers() {
        assert_eq!(
            key_event_to_string(&KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL | KeyModifiers::ALT)),
            "ctrl-alt-a".to_string()
        );
    }

    #[test]
    fn test_invalid_keys() {
        assert!(parse_key_event("invalid-key").is_err());
        assert!(parse_key_event("ctrl-invalid-key").is_err());
    }

    #[test]
    fn test_case_insensitivity() {
        assert_eq!(parse_key_event("CTRL-a").unwrap(), KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));

        assert_eq!(parse_key_event("AlT-eNtEr").unwrap(), KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT));
    }
}
