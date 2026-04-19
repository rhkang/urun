use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    pub editor_root: Option<PathBuf>,
}

impl Config {
    pub fn editor_root(&self) -> PathBuf {
        self.editor_root.clone().unwrap_or_else(platform_default)
    }
}

pub fn load() -> Config {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(s) => toml::from_str(&s)
            .unwrap_or_else(|e| crate::fatal(format!("{}: {}", path.display(), e))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
        Err(e) => crate::fatal(format!("{}: {}", path.display(), e)),
    }
}

pub fn uproxy_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| crate::fatal("could not determine home directory"))
        .join(".uproxy")
}

fn config_path() -> PathBuf {
    uproxy_dir().join("config.toml")
}

fn platform_default() -> PathBuf {
    #[cfg(windows)]
    {
        PathBuf::from(r"C:\Program Files\Unity\Hub\Editor")
    }
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/Applications/Unity/Hub/Editor")
    }
    #[cfg(target_os = "linux")]
    {
        dirs::home_dir()
            .unwrap_or_else(|| crate::fatal("could not determine home directory"))
            .join("Unity/Hub/Editor")
    }
}
