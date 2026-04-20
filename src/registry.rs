use std::env;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::config::{self, ConfigError, Project};

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("unknown alias: {0}")]
    UnknownAlias(String),
    #[error("alias already registered: {0}")]
    AliasExists(String),
    #[error("not a directory: {}", .0.display())]
    ProjectPathNotDirectory(PathBuf),
}

pub type Result<T> = std::result::Result<T, RegistryError>;

pub fn lookup(alias: &str) -> Result<PathBuf> {
    config::load()?
        .projects
        .into_iter()
        .find(|p| p.alias == alias)
        .map(|p| p.path)
        .ok_or_else(|| RegistryError::UnknownAlias(alias.to_string()))
}

pub fn add(alias: &str, path: &Path) -> Result<()> {
    let abs = absolutize(path);
    if !abs.is_dir() {
        return Err(RegistryError::ProjectPathNotDirectory(abs));
    }

    let mut cfg = config::load()?;
    if cfg.projects.iter().any(|p| p.alias == alias) {
        return Err(RegistryError::AliasExists(alias.to_string()));
    }
    cfg.projects.push(Project {
        alias: alias.to_string(),
        path: abs,
    });
    config::save(&cfg)?;
    Ok(())
}

pub fn remove(alias: &str) -> Result<()> {
    let mut cfg = config::load()?;
    let before = cfg.projects.len();
    cfg.projects.retain(|p| p.alias != alias);
    if cfg.projects.len() == before {
        return Err(RegistryError::UnknownAlias(alias.to_string()));
    }
    config::save(&cfg)?;
    Ok(())
}

pub fn list() -> Result<()> {
    let cfg = config::load()?;
    if cfg.projects.is_empty() {
        println!("(no projects registered)");
        return Ok(());
    }
    let width = cfg
        .projects
        .iter()
        .map(|p| p.alias.len())
        .max()
        .unwrap_or(0);
    for p in &cfg.projects {
        println!("{:<width$}  {}", p.alias, p.path.display(), width = width);
    }
    Ok(())
}

pub fn load_projects() -> Result<Vec<Project>> {
    Ok(config::load()?.projects)
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
