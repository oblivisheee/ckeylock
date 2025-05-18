use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    pub bind: String,
    pub password: Option<String>,
    pub dump_password: String,
    pub dump_path: String,
    pub workers: Option<usize>,
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
}
#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("File system error: {0}")]
    Fs(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("Config not found")]
    NotFound,
}
