use std::path::PathBuf;

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not determine home directory")]
    HomeDirUnavailable,
    #[error("failed to read {}", .path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse {}", .path.display())]
    Deserialize {
        path: PathBuf,
        #[source]
        source: Box<toml::de::Error>,
    },
}

pub type Result<T> = std::result::Result<T, ConfigError>;

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    pub editor_root: Option<PathBuf>,
}

impl Config {
    pub fn editor_root(&self) -> Result<PathBuf> {
        match &self.editor_root {
            Some(p) => Ok(p.clone()),
            None => platform_default(),
        }
    }
}

pub fn load() -> Result<Config> {
    let path = config_path()?;
    match std::fs::read_to_string(&path) {
        Ok(s) => toml::from_str(&s).map_err(|e| ConfigError::Deserialize {
            path,
            source: Box::new(e),
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(source) => Err(ConfigError::Read { path, source }),
    }
}

pub fn urun_dir() -> Result<PathBuf> {
    Ok(dirs::config_local_dir()
        .ok_or(ConfigError::HomeDirUnavailable)?
        .join("urun"))
}

fn config_path() -> Result<PathBuf> {
    Ok(urun_dir()?.join("config.toml"))
}

fn platform_default() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        Ok(PathBuf::from(r"C:\Program Files\Unity\Hub\Editor"))
    }
    #[cfg(target_os = "macos")]
    {
        Ok(PathBuf::from("/Applications/Unity/Hub/Editor"))
    }
    #[cfg(target_os = "linux")]
    {
        Ok(dirs::home_dir()
            .ok_or(ConfigError::HomeDirUnavailable)?
            .join("Unity/Hub/Editor"))
    }
}
