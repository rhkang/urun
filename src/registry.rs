use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum RegistryError {
    Io(std::io::Error),
    Parse(String),
    UnknownAlias(String),
    AliasExists(String),
    NotADirectory(PathBuf),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::Io(e) => write!(f, "registry I/O error: {}", e),
            RegistryError::Parse(e) => write!(f, "registry parse error: {}", e),
            RegistryError::UnknownAlias(a) => write!(f, "unknown alias: {}", a),
            RegistryError::AliasExists(a) => write!(f, "alias already registered: {}", a),
            RegistryError::NotADirectory(p) => write!(f, "not a directory: {}", p.display()),
        }
    }
}

impl From<std::io::Error> for RegistryError {
    fn from(e: std::io::Error) -> Self {
        RegistryError::Io(e)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Registry {
    #[serde(default)]
    projects: BTreeMap<String, PathBuf>,
}

pub fn lookup(alias: &str) -> Result<PathBuf, RegistryError> {
    load()?
        .projects
        .get(alias)
        .cloned()
        .ok_or_else(|| RegistryError::UnknownAlias(alias.to_string()))
}

pub fn add(alias: &str, path: &Path) -> Result<(), RegistryError> {
    let abs = absolutize(path);
    if !abs.is_dir() {
        return Err(RegistryError::NotADirectory(abs));
    }

    let mut reg = load()?;
    if reg.projects.contains_key(alias) {
        return Err(RegistryError::AliasExists(alias.to_string()));
    }
    reg.projects.insert(alias.to_string(), abs);
    save(&reg)
}

pub fn remove(alias: &str) -> Result<(), RegistryError> {
    let mut reg = load()?;
    if reg.projects.remove(alias).is_none() {
        return Err(RegistryError::UnknownAlias(alias.to_string()));
    }
    save(&reg)
}

pub fn list() -> Result<(), RegistryError> {
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

fn registry_path() -> PathBuf {
    crate::config::uproxy_dir().join("projects.toml")
}

fn load() -> Result<Registry, RegistryError> {
    let path = registry_path();
    match std::fs::read_to_string(&path) {
        Ok(s) => toml::from_str(&s).map_err(|e| RegistryError::Parse(e.to_string())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Registry::default()),
        Err(e) => Err(RegistryError::Io(e)),
    }
}

fn save(reg: &Registry) -> Result<(), RegistryError> {
    let dir = crate::config::uproxy_dir();
    std::fs::create_dir_all(&dir)?;
    let body = toml::to_string_pretty(reg).map_err(|e| RegistryError::Parse(e.to_string()))?;
    std::fs::write(registry_path(), body)?;
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
