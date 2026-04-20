use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::config::{self, ConfigError};
use crate::registry;

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Registry(#[from] registry::RegistryError),
    #[error("ProjectVersion.txt not found: {}", .0.display())]
    VersionFileMissing(PathBuf),
    #[error("failed to read {}", .path.display())]
    VersionFileUnreadable {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("m_EditorVersion missing in {}", .0.display())]
    VersionFieldMissing(PathBuf),
    #[error("Unity {version} not installed (expected at {})", .path.display())]
    EditorNotFound { version: String, path: PathBuf },
}

pub type Result<T> = std::result::Result<T, ResolveError>;

pub struct Resolved {
    pub unity: PathBuf,
    pub project: PathBuf,
}

cfg_select! {
    windows => {
        const UNITY_REL: &str = "Editor/Unity.exe";
    }

    target_os = "linux" => {
        const UNITY_REL: &str = "Editor/Unity";
    }

    target_os = "macos" => {
        const UNITY_REL: &str = "Unity.app/Contents/MacOS/Unity";
    }

    _ => compile_error!{ "Not supported platform" },
}

pub fn resolve(alias: &str) -> Result<Resolved> {
    let project = registry::lookup(alias)?;
    let version = read_project_version(&project)?;
    let root = config::load()?.editor_root()?;
    let unity = root.join(&version).join(UNITY_REL);
    if !unity.exists() {
        return Err(ResolveError::EditorNotFound {
            version,
            path: unity,
        });
    }
    Ok(Resolved { unity, project })
}

fn read_project_version(project: &Path) -> Result<String> {
    let path = project.join("ProjectSettings").join("ProjectVersion.txt");
    let contents = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(ResolveError::VersionFileMissing(path));
        }
        Err(source) => return Err(ResolveError::VersionFileUnreadable { path, source }),
    };
    for line in contents.lines() {
        if let Some(rest) = line.strip_prefix("m_EditorVersion:") {
            let v = rest.trim();
            if !v.is_empty() {
                return Ok(v.to_string());
            }
        }
    }
    Err(ResolveError::VersionFieldMissing(path))
}
