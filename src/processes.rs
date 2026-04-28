use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use sysinfo::{Pid, ProcessesToUpdate, System};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("no process with pid {0}")]
    NotFound(u32),
    #[error("failed to kill pid {0}")]
    KillFailed(u32),
}

pub type Result<T> = std::result::Result<T, ProcessError>;

pub struct RunningUnity {
    pub pid: u32,
    pub project: Option<PathBuf>,
}

pub fn running() -> Vec<RunningUnity> {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    let unity_procs: Vec<(u32, Option<u32>)> = sys
        .processes()
        .iter()
        .filter(|(_, p)| is_unity(&p.name().to_string_lossy()))
        .map(|(pid, p)| (pid.as_u32(), p.parent().map(|pp| pp.as_u32())))
        .collect();

    if unity_procs.is_empty() {
        return Vec::new();
    }

    let unity_pids: HashSet<u32> = unity_procs.iter().map(|(pid, _)| *pid).collect();

    // Skip Unity's own subprocesses (e.g. Asset Import Worker) — only show
    // top-level editors. Killing the parent terminates its workers.
    let top_pids: Vec<u32> = unity_procs
        .iter()
        .filter(|(_, ppid)| ppid.is_none_or(|pp| !unity_pids.contains(&pp)))
        .map(|(pid, _)| *pid)
        .collect();

    if top_pids.is_empty() {
        return Vec::new();
    }

    let cmdlines = read_cmdlines(&top_pids);
    let mut out: Vec<RunningUnity> = top_pids
        .into_iter()
        .map(|pid| {
            let project = cmdlines.get(&pid).and_then(|args| extract_project_path(args));
            RunningUnity { pid, project }
        })
        .collect();
    out.sort_by_key(|r| r.pid);
    out
}

pub fn kill_pid(pid: u32) -> Result<()> {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    match sys.process(Pid::from_u32(pid)) {
        Some(p) => {
            if p.kill() {
                Ok(())
            } else {
                Err(ProcessError::KillFailed(pid))
            }
        }
        None => Err(ProcessError::NotFound(pid)),
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

fn extract_project_path(args: &[String]) -> Option<PathBuf> {
    let mut iter = args.iter();
    iter.next(); // skip argv[0]
    while let Some(arg) = iter.next() {
        if arg.eq_ignore_ascii_case("-projectpath") {
            return iter.next().map(PathBuf::from);
        }
        if let Some(rest) = strip_prefix_ci(arg, "-projectpath=") {
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

// sysinfo::Process::cmd() returns an empty slice for Unity on Windows and
// macOS even when the data is plainly readable via WMI / sysctl. Read
// command lines natively instead.

#[cfg(windows)]
fn read_cmdlines(_pids: &[u32]) -> HashMap<u32, Vec<String>> {
    let mut map = HashMap::new();
    let script = "Get-CimInstance Win32_Process -Filter \"Name='Unity.exe'\" | \
                  ForEach-Object { \"$($_.ProcessId)`t$($_.CommandLine)\" }";
    let output = match std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", script])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return map,
    };
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let line = line.trim_end();
        let Some((pid_str, cmdline)) = line.split_once('\t') else {
            continue;
        };
        let Ok(pid) = pid_str.trim().parse::<u32>() else {
            continue;
        };
        map.insert(pid, tokenize_cmdline(cmdline));
    }
    map
}

#[cfg(target_os = "macos")]
fn read_cmdlines(pids: &[u32]) -> HashMap<u32, Vec<String>> {
    let mut map = HashMap::new();
    for &pid in pids {
        let output = match std::process::Command::new("ps")
            .args(["-o", "command=", "-p", &pid.to_string()])
            .output()
        {
            Ok(o) if o.status.success() => o,
            _ => continue,
        };
        let line = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if line.is_empty() {
            continue;
        }
        let args: Vec<String> = line.split_whitespace().map(String::from).collect();
        map.insert(pid, args);
    }
    map
}

#[cfg(target_os = "linux")]
fn read_cmdlines(pids: &[u32]) -> HashMap<u32, Vec<String>> {
    let mut map = HashMap::new();
    for &pid in pids {
        let raw = match std::fs::read(format!("/proc/{}/cmdline", pid)) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let args: Vec<String> = raw
            .split(|&b| b == 0)
            .filter(|s| !s.is_empty())
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect();
        if !args.is_empty() {
            map.insert(pid, args);
        }
    }
    map
}

#[cfg(windows)]
fn tokenize_cmdline(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quote = false;
    for c in s.chars() {
        match c {
            '"' => in_quote = !in_quote,
            c if c.is_whitespace() && !in_quote => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}
