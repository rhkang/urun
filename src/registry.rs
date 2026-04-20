use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::{self, ConfigError};

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error(transparent)]
    Config(#[from] ConfigError),
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
    #[error("failed to serialize registry")]
    Serialize(#[source] Box<toml::ser::Error>),
    #[error("unknown alias: {0}")]
    UnknownAlias(String),
    #[error("alias already registered: {0}")]
    AliasExists(String),
    #[error("not a directory: {}", .0.display())]
    ProjectPathNotDirectory(PathBuf),
}

pub type Result<T> = std::result::Result<T, RegistryError>;

#[derive(Debug, Default, Serialize, Deserialize)]
struct Registry {
    #[serde(default)]
    projects: BTreeMap<String, PathBuf>,
}

pub fn lookup(alias: &str) -> Result<PathBuf> {
    load()?
        .projects
        .get(alias)
        .cloned()
        .ok_or_else(|| RegistryError::UnknownAlias(alias.to_string()))
}

pub fn add(alias: &str, path: &Path) -> Result<()> {
    let abs = absolutize(path);
    if !abs.is_dir() {
        return Err(RegistryError::ProjectPathNotDirectory(abs));
    }

    let mut reg = load()?;
    if reg.projects.contains_key(alias) {
        return Err(RegistryError::AliasExists(alias.to_string()));
    }
    reg.projects.insert(alias.to_string(), abs);
    save(&reg)
}

pub fn remove(alias: &str) -> Result<()> {
    let mut reg = load()?;
    if reg.projects.remove(alias).is_none() {
        return Err(RegistryError::UnknownAlias(alias.to_string()));
    }
    save(&reg)
}

pub fn list() -> Result<()> {
    let reg = load()?;
    if reg.projects.is_empty() {
        println!("(no projects registered)");
        return Ok(());
    }
    let width = reg.projects.keys().map(String::len).max().unwrap_or(0);
    for (alias, path) in &reg.projects {
        println!("{:<width$}  {}", alias, path.display(), width = width);
    }
    Ok(())
}

pub fn load_projects() -> Result<BTreeMap<String, PathBuf>> {
    Ok(load()?.projects)
}

fn registry_path() -> Result<PathBuf> {
    Ok(config::urun_dir()?.join("projects.toml"))
}

fn load() -> Result<Registry> {
    let path = registry_path()?;
    match std::fs::read_to_string(&path) {
        Ok(s) => toml::from_str(&s).map_err(|e| RegistryError::Deserialize {
            path,
            source: Box::new(e),
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Registry::default()),
        Err(source) => Err(RegistryError::Read { path, source }),
    }
}

fn save(reg: &Registry) -> Result<()> {
    let dir = config::urun_dir()?;
    std::fs::create_dir_all(&dir).map_err(|source| RegistryError::Write {
        path: dir.clone(),
        source,
    })?;
    let body = toml::to_string_pretty(reg).map_err(|e| RegistryError::Serialize(Box::new(e)))?;
    let path = registry_path()?;
    std::fs::write(&path, body).map_err(|source| RegistryError::Write { path, source })?;
    Ok(())
}

fn absolutize(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize() {
        let reg = Registry {
            projects: BTreeMap::from([
                ("foo".to_string(), PathBuf::from("/path/to/foo")),
                ("bar".to_string(), PathBuf::from("/path/to/bar")),
            ]),
        };
        let s = toml::to_string_pretty(&reg).unwrap();
        assert_eq!(
            s,
            r#"projects = { bar = "/path/to/bar", foo = "/path/to/foo" }"#
        );

        let deserialized = toml::from_str::<Registry>(&s).unwrap();
        assert_eq!(reg.projects, deserialized.projects);
    }
}
