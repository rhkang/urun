use std::ffi::OsString;
use std::path::{Path, PathBuf};

use sysinfo::{Pid, ProcessesToUpdate, System};

pub struct RunningUnity {
    pub pid: u32,
    pub project: Option<PathBuf>,
}

pub fn running() -> Vec<RunningUnity> {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    let mut out = Vec::new();
    for (pid, proc_) in sys.processes() {
        let name = proc_.name().to_string_lossy();
        if !is_unity(&name) {
            continue;
        }
        out.push(RunningUnity {
            pid: pid.as_u32(),
            project: extract_project_path(proc_.cmd()),
        });
    }
    out.sort_by_key(|r| r.pid);
    out
}

pub fn kill_pid(pid: u32) -> Result<(), String> {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    match sys.process(Pid::from_u32(pid)) {
        Some(p) => {
            if p.kill() {
                Ok(())
            } else {
                Err(format!("failed to kill pid {}", pid))
            }
        }
        None => Err(format!("no process with pid {}", pid)),
    }
}

pub fn path_matches(a: &Path, b: &Path) -> bool {
    normalize(a) == normalize(b)
}

fn normalize(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    let trimmed = s.trim_end_matches('/').to_string();
    if cfg!(windows) {
        trimmed.to_ascii_lowercase()
    } else {
        trimmed
    }
}

fn is_unity(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == "unity.exe" || lower == "unity"
}

fn extract_project_path(cmd: &[OsString]) -> Option<PathBuf> {
    let mut iter = cmd.iter();
    iter.next(); // skip argv[0]
    while let Some(arg) = iter.next() {
        let s = arg.to_string_lossy();
        if s.eq_ignore_ascii_case("-projectpath") {
            return iter.next().map(PathBuf::from);
        }
        if let Some(rest) = strip_prefix_ci(&s, "-projectpath=") {
            return Some(PathBuf::from(rest));
        }
    }
    None
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() < prefix.len() {
        return None;
    }
    let (head, tail) = s.split_at(prefix.len());
    if head.eq_ignore_ascii_case(prefix) {
        Some(tail)
    } else {
        None
    }
}
