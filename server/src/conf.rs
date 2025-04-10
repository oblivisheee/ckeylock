use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    pub bind: String,
    pub password: Option<String>,
    pub dump_password: String,
    pub dump_path: String,
}

impl Config {
    pub fn from_toml(path: &str) -> Result<Self, ConfigError> {
        let data = match std::fs::read_to_string(path) {
            Ok(data) => data,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    return Err(ConfigError::NotFound);
                } else {
                    return Err(ConfigError::Fs(e));
                }
            }
        };
        let config: Config = toml::from_str(&data)?;
        Ok(config)
    }
    pub fn from_ron(path: &str) -> Result<Self, ConfigError> {
        let data = match std::fs::read_to_string(path) {
            Ok(data) => data,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    return Err(ConfigError::NotFound);
                } else {
                    return Err(ConfigError::Fs(e));
                }
            }
        };
        let config: Config = ron::from_str(&data)?;
        Ok(config)
    }
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).unwrap()
    }
    pub fn to_ron(&self) -> String {
        ron::to_string(self).unwrap()
    }
}
#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("File system error: {0}")]
    Fs(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("RON parse error: {0}")]
    Ron(#[from] ron::de::SpannedError),
    #[error("Config not found")]
    NotFound,
}
