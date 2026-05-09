use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml::Value;
use thiserror::Error;
use yoda_core::default_color_for_class;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YoDaSettings {
    pub image_base_path: PathBuf,
    pub label_base_path: PathBuf,
    pub class_info: Option<PathBuf>,
    pub color_map: Option<PathBuf>,
    pub host: Option<String>,
    pub port: u16,
}

impl Default for YoDaSettings {
    fn default() -> Self {
        Self {
            image_base_path: PathBuf::from("example_data/images"),
            label_base_path: PathBuf::from("example_data/labels"),
            class_info: Some(PathBuf::from("example_data/carparts-seg.yaml")),
            color_map: None,
            host: None,
            port: 8080,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct YoDaSettingsOverrides {
    pub image_base_path: Option<PathBuf>,
    pub label_base_path: Option<PathBuf>,
    pub class_info: Option<PathBuf>,
    pub color_map: Option<PathBuf>,
    pub host: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YoDaConfig {
    pub settings: YoDaSettings,
    pub color_map_tuples: HashMap<u32, (u8, u8, u8)>,
    pub color_map_strings: HashMap<u32, String>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid YODA_PORT value {value:?}")]
    InvalidPort { value: String },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
}

impl YoDaSettings {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_env_iter(env::vars())
    }

    pub fn from_env_iter<I, K, V>(vars: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let mut settings = Self::default();

        for (key, value) in vars {
            let key = key.into();
            let value = value.into();
            match key.as_str() {
                "YODA_IMAGE_BASE_PATH" => settings.image_base_path = PathBuf::from(value),
                "YODA_LABEL_BASE_PATH" => settings.label_base_path = PathBuf::from(value),
                "YODA_CLASS_INFO" | "YODA_CLASS_INFO_YAML" => {
                    settings.class_info = Some(PathBuf::from(value));
                }
                "YODA_COLOR_MAP" | "YODA_COLOR_MAP_YAML" => {
                    settings.color_map = Some(PathBuf::from(value));
                }
                "YODA_HOST" => settings.host = Some(value),
                "YODA_PORT" => {
                    settings.port = value
                        .parse::<u16>()
                        .map_err(|_| ConfigError::InvalidPort { value })?;
                }
                _ => {}
            }
        }

        Ok(settings)
    }

    pub fn with_overrides(mut self, overrides: YoDaSettingsOverrides) -> Self {
        if let Some(path) = overrides.image_base_path {
            self.image_base_path = path;
        }
        if let Some(path) = overrides.label_base_path {
            self.label_base_path = path;
        }
        if let Some(path) = overrides.class_info {
            self.class_info = Some(path);
        }
        if let Some(path) = overrides.color_map {
            self.color_map = Some(path);
        }
        if let Some(host) = overrides.host {
            self.host = Some(host);
        }
        if let Some(port) = overrides.port {
            self.port = port;
        }

        self
    }
}

impl YoDaConfig {
    pub fn load() -> Result<Self, ConfigError> {
        Self::load_with_overrides(YoDaSettingsOverrides::default())
    }

    pub fn load_with_overrides(overrides: YoDaSettingsOverrides) -> Result<Self, ConfigError> {
        let settings = YoDaSettings::from_env()?.with_overrides(overrides);
        let (color_map_tuples, color_map_strings) = load_color_map(settings.color_map.as_deref())?;

        Ok(Self {
            settings,
            color_map_tuples,
            color_map_strings,
        })
    }

    pub fn get_color_tuple(&self, class_id: u32) -> (u8, u8, u8) {
        self.color_map_tuples
            .get(&class_id)
            .copied()
            .unwrap_or((255, 255, 255))
    }

    pub fn get_color_string(&self, class_id: u32) -> String {
        self.color_map_strings
            .get(&class_id)
            .cloned()
            .unwrap_or_else(|| String::from("rgb(255,255,255)"))
    }
}

pub fn load_class_map(class_info_yaml: Option<&Path>) -> HashMap<u32, String> {
    let Some(path) = class_info_yaml else {
        return HashMap::new();
    };
    if !path.exists() {
        return HashMap::new();
    }

    let Ok(contents) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    let Ok(document) = serde_yaml::from_str::<Value>(&contents) else {
        return HashMap::new();
    };
    let Some(names) = document.get("names") else {
        return HashMap::new();
    };

    parse_class_names(names)
}

pub fn load_color_map(
    color_map_yaml: Option<&Path>,
) -> Result<(HashMap<u32, (u8, u8, u8)>, HashMap<u32, String>), ConfigError> {
    let mut tuples = HashMap::new();
    let mut strings = HashMap::new();
    for class_id in 0..100 {
        let rgb = default_color_for_class(class_id);
        tuples.insert(class_id, rgb);
        strings.insert(class_id, format!("rgb({},{},{})", rgb.0, rgb.1, rgb.2));
    }

    let Some(path) = color_map_yaml else {
        return Ok((tuples, strings));
    };
    if !path.exists() {
        return Ok((tuples, strings));
    }

    let contents = fs::read_to_string(path)?;
    let document = serde_yaml::from_str::<Value>(&contents)?;
    if let Value::Mapping(entries) = document {
        for (key, value) in entries {
            let Some(class_id) = yaml_key_to_u32(&key) else {
                continue;
            };
            if let Some(rgb) = yaml_value_to_rgb(&value) {
                tuples.insert(class_id, rgb);
                strings.insert(class_id, format!("rgb({},{},{})", rgb.0, rgb.1, rgb.2));
            }
        }
    }

    Ok((tuples, strings))
}

fn parse_class_names(names: &Value) -> HashMap<u32, String> {
    match names {
        Value::Mapping(mapping) => mapping
            .iter()
            .filter_map(|(key, value)| {
                let class_id = yaml_key_to_u32(key)?;
                let name = value.as_str()?.to_string();
                Some((class_id, name))
            })
            .collect(),
        Value::Sequence(sequence) => sequence
            .iter()
            .enumerate()
            .filter_map(|(index, value)| value.as_str().map(|name| (index as u32, name.to_string())))
            .collect(),
        _ => HashMap::new(),
    }
}

fn yaml_key_to_u32(value: &Value) -> Option<u32> {
    match value {
        Value::Number(number) => number.as_u64().map(|value| value as u32),
        Value::String(text) => text.parse::<u32>().ok(),
        _ => None,
    }
}

fn yaml_value_to_rgb(value: &Value) -> Option<(u8, u8, u8)> {
    match value {
        Value::Sequence(sequence) if sequence.len() == 3 => Some((
            sequence[0].as_u64()? as u8,
            sequence[1].as_u64()? as u8,
            sequence[2].as_u64()? as u8,
        )),
        Value::String(text) => parse_hex_color(text),
        _ => None,
    }
}

fn parse_hex_color(value: &str) -> Option<(u8, u8, u8)> {
    let value = value.strip_prefix('#')?;
    if value.len() != 6 {
        return None;
    }

    let red = u8::from_str_radix(&value[0..2], 16).ok()?;
    let green = u8::from_str_radix(&value[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&value[4..6], 16).ok()?;
    Some((red, green, blue))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::{load_class_map, YoDaConfig, YoDaSettings, YoDaSettingsOverrides};

    #[test]
    fn defaults() {
        let settings = YoDaSettings::from_env_iter(Vec::<(String, String)>::new()).expect("load settings");
        assert_eq!(settings.image_base_path, PathBuf::from("example_data/images"));
        assert_eq!(settings.label_base_path, PathBuf::from("example_data/labels"));
        assert_eq!(settings.port, 8080);
        assert_eq!(settings.color_map, None);
    }

    #[test]
    fn env_prefix() {
        let settings = YoDaSettings::from_env_iter(vec![("YODA_PORT", "9999")]).expect("load settings");
        assert_eq!(settings.port, 9999);
    }

    #[test]
    fn load_creates_config() {
        let config = YoDaConfig::load().expect("load config");
        assert_eq!(config.settings.port, 8080);
    }

    #[test]
    fn load_with_overrides() {
        let config = YoDaConfig::load_with_overrides(YoDaSettingsOverrides {
            port: Some(1234),
            ..YoDaSettingsOverrides::default()
        })
        .expect("load config");
        assert_eq!(config.settings.port, 1234);
    }

    #[test]
    fn default_color_map_has_100_entries() {
        let config = YoDaConfig::load().expect("load config");
        assert!(config.color_map_tuples.len() >= 100);
        assert!(config.color_map_strings.len() >= 100);
    }

    #[test]
    fn custom_color_map_overrides() {
        let temp = TempDir::new().expect("create temp dir");
        let yaml = temp.path().join("colors.yaml");
        fs::write(&yaml, "0: [255, 0, 0]\n1: [0, 255, 0]\n").expect("write colors");
        let config = YoDaConfig::load_with_overrides(YoDaSettingsOverrides {
            color_map: Some(yaml),
            ..YoDaSettingsOverrides::default()
        })
        .expect("load config");
        assert_eq!(config.color_map_tuples[&0], (255, 0, 0));
        assert_eq!(config.get_color_string(0), "rgb(255,0,0)");
        assert_eq!(config.color_map_tuples[&1], (0, 255, 0));
        assert!(config.color_map_tuples.contains_key(&50));
    }

    #[test]
    fn get_color_string_fallback() {
        let config = YoDaConfig::load().expect("load config");
        assert_eq!(config.get_color_string(9999), "rgb(255,255,255)");
    }

    #[test]
    fn get_color_tuple_fallback() {
        let config = YoDaConfig::load().expect("load config");
        assert_eq!(config.get_color_tuple(9999), (255, 255, 255));
    }

    #[test]
    fn load_valid_yaml() {
        let temp = TempDir::new().expect("create temp dir");
        let yaml = temp.path().join("classes.yaml");
        fs::write(
            &yaml,
            "names:\n  0: back_bumper\n  1: front_bumper\n  2: wheel\n",
        )
        .expect("write classes");
        let class_map = load_class_map(Some(&yaml));
        assert_eq!(class_map[&0], "back_bumper");
        assert_eq!(class_map[&1], "front_bumper");
        assert_eq!(class_map[&2], "wheel");
    }

    #[test]
    fn load_none() {
        assert!(load_class_map(None).is_empty());
    }

    #[test]
    fn load_nonexistent() {
        let temp = TempDir::new().expect("create temp dir");
        assert!(load_class_map(Some(&temp.path().join("nope.yaml"))).is_empty());
    }

    #[test]
    fn class_map_sequence_format_is_supported() {
        let temp = TempDir::new().expect("create temp dir");
        let yaml = temp.path().join("classes.yaml");
        fs::write(&yaml, "names:\n  - wheel\n  - bumper\n").expect("write classes");
        let class_map = load_class_map(Some(&yaml));
        assert_eq!(class_map[&0], "wheel");
        assert_eq!(class_map[&1], "bumper");
    }

    #[test]
    fn bad_yaml_returns_empty_class_map() {
        let temp = TempDir::new().expect("create temp dir");
        let yaml = temp.path().join("classes.yaml");
        fs::write(&yaml, "names: [\n").expect("write invalid yaml");
        assert!(load_class_map(Some(&yaml)).is_empty());
    }
}