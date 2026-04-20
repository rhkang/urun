use std::fmt;
use std::path::{Path, PathBuf};

use crate::{config, registry};

#[derive(Debug)]
pub enum ResolveError {
    Registry(registry::RegistryError),
    VersionFileMissing(PathBuf),
    VersionFileUnreadable(PathBuf, std::io::Error),
    VersionFieldMissing(PathBuf),
    EditorNotFound { version: String, path: PathBuf },
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolveError::Registry(e) => write!(f, "{}", e),
            ResolveError::VersionFileMissing(p) => {
                write!(f, "ProjectVersion.txt not found: {}", p.display())
            }
            ResolveError::VersionFileUnreadable(p, e) => {
                write!(f, "could not read {}: {}", p.display(), e)
            }
            ResolveError::VersionFieldMissing(p) => {
                write!(f, "m_EditorVersion missing in {}", p.display())
            }
            ResolveError::EditorNotFound { version, path } => {
                write!(f, "Unity {} not installed (expected at {})", version, path.display())
            }
        }
    }
}

impl From<registry::RegistryError> for ResolveError {
    fn from(e: registry::RegistryError) -> Self {
        ResolveError::Registry(e)
    }
}

pub struct Resolved {
    pub unity: PathBuf,
    pub project: PathBuf,
}

#[cfg(windows)]
const UNITY_REL: &str = "Editor/Unity.exe";
#[cfg(target_os = "linux")]
const UNITY_REL: &str = "Editor/Unity";
#[cfg(target_os = "macos")]
const UNITY_REL: &str = "Unity.app/Contents/MacOS/Unity";

pub fn resolve(alias: &str) -> Result<Resolved, ResolveError> {
    let project = registry::lookup(alias)?;
    let version = read_project_version(&project)?;
    let root = config::load().editor_root();
    let unity = root.join(&version).join(UNITY_REL);
    if !unity.exists() {
        return Err(ResolveError::EditorNotFound { version, path: unity });
    }
    Ok(Resolved { unity, project })
}

fn read_project_version(project: &Path) -> Result<String, ResolveError> {
    let path = project.join("ProjectSettings").join("ProjectVersion.txt");
    let contents = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(ResolveError::VersionFileMissing(path));
        }
        Err(e) => return Err(ResolveError::VersionFileUnreadable(path, e)),
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
