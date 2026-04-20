use std::path::PathBuf;

use serde::{Deserialize, Serialize};
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
    #[error("failed to write {}", .path.display())]
    Write {
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
    #[error("failed to serialize config")]
    Serialize(#[source] Box<toml::ser::Error>),
}

pub type Result<T> = std::result::Result<T, ConfigError>;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor_root: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub projects: Vec<Project>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub alias: String,
    pub path: PathBuf,
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

pub fn save(cfg: &Config) -> Result<()> {
    let dir = urun_dir()?;
    std::fs::create_dir_all(&dir).map_err(|source| ConfigError::Write {
        path: dir.clone(),
        source,
    })?;
    let body = toml::to_string_pretty(cfg).map_err(|e| ConfigError::Serialize(Box::new(e)))?;
    let path = config_path()?;
    std::fs::write(&path, body).map_err(|source| ConfigError::Write { path, source })?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_full() {
        let cfg = Config {
            editor_root: Some(PathBuf::from("/opt/unity")),
            projects: vec![
                Project {
                    alias: "foo".into(),
                    path: PathBuf::from("/path/to/foo"),
                },
                Project {
                    alias: "bar".into(),
                    path: PathBuf::from("/path/to/bar"),
                },
            ],
        };
        let s = toml::to_string_pretty(&cfg).unwrap();
        let back: Config = toml::from_str(&s).unwrap();

        assert_eq!(back.editor_root, cfg.editor_root);
        assert_eq!(back.projects.len(), 2);
        // Order is preserved (not sorted alphabetically).
        assert_eq!(back.projects[0].alias, "foo");
        assert_eq!(back.projects[1].alias, "bar");
    }

    #[test]
    fn deserialize_empty_is_default() {
        let cfg: Config = toml::from_str("").unwrap();
        assert!(cfg.editor_root.is_none());
        assert!(cfg.projects.is_empty());
    }

    #[test]
    fn deserialize_only_projects() {
        let s = r#"
            [[projects]]
            alias = "a"
            path = "/a"
        "#;
        let cfg: Config = toml::from_str(s).unwrap();
        assert!(cfg.editor_root.is_none());
        assert_eq!(cfg.projects.len(), 1);
        assert_eq!(cfg.projects[0].alias, "a");
    }
}
