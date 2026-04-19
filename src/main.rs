mod config;
mod registry;
mod resolver;

use std::env;
use std::ffi::OsString;
use std::fmt::Display;
use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "\
urun — project-aware Unity launcher

USAGE:
    urun <alias> [unity-args…]        launch Unity for a registered project
    urun add <alias> <project-path>   register a project
    urun remove <alias>               unregister a project
    urun list                         list all registered projects
    urun which <alias>                print resolved Unity.exe path
    urun --version                    print urun version";

fn main() -> ExitCode {
    let mut args: Vec<OsString> = env::args_os().skip(1).collect();
    if args.is_empty() {
        eprintln!("{}", USAGE);
        return ExitCode::from(2);
    }

    let first = args.remove(0);
    match first.to_str() {
        Some("--version") | Some("-V") => {
            println!("urun {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("--help") | Some("-h") | Some("help") => {
            println!("{}", USAGE);
            ExitCode::SUCCESS
        }
        Some("add") => cmd_add(&args),
        Some("remove") => cmd_remove(&args),
        Some("list") => cmd_list(&args),
        Some("which") => cmd_which(&args),
        _ => {
            let alias = first.to_string_lossy().into_owned();
            cmd_launch(&alias, &args)
        }
    }
}

fn cmd_add(args: &[OsString]) -> ExitCode {
    if args.len() != 2 {
        eprintln!("usage: urun add <alias> <project-path>");
        return ExitCode::from(2);
    }
    let alias = args[0].to_string_lossy();
    let path = Path::new(&args[1]);
    match registry::add(&alias, path) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fatal_code(e),
    }
}

fn cmd_remove(args: &[OsString]) -> ExitCode {
    if args.len() != 1 {
        eprintln!("usage: urun remove <alias>");
        return ExitCode::from(2);
    }
    let alias = args[0].to_string_lossy();
    match registry::remove(&alias) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fatal_code(e),
    }
}

fn cmd_list(args: &[OsString]) -> ExitCode {
    if !args.is_empty() {
        eprintln!("usage: urun list");
        return ExitCode::from(2);
    }
    match registry::list() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fatal_code(e),
    }
}

fn cmd_which(args: &[OsString]) -> ExitCode {
    if args.len() != 1 {
        eprintln!("usage: urun which <alias>");
        return ExitCode::from(2);
    }
    let alias = args[0].to_string_lossy();
    match resolver::resolve(&alias) {
        Ok(r) => {
            println!("{}", r.unity.display());
            ExitCode::SUCCESS
        }
        Err(e) => fatal_code(e),
    }
}

fn cmd_launch(alias: &str, rest: &[OsString]) -> ExitCode {
    let resolved = match resolver::resolve(alias) {
        Ok(r) => r,
        Err(e) => return fatal_code(e),
    };
    exec(&resolved.unity, &resolved.project, rest)
}

fn fatal_code<E: Display>(e: E) -> ExitCode {
    eprintln!("urun: {}", e);
    ExitCode::from(1)
}

pub(crate) fn fatal<E: Display>(e: E) -> ! {
    eprintln!("urun: {}", e);
    std::process::exit(1);
}

#[cfg(unix)]
fn exec(unity: &Path, project: &Path, rest: &[OsString]) -> ExitCode {
    use std::os::unix::process::CommandExt;
    let err = std::process::Command::new(unity)
        .arg("-projectPath")
        .arg(project)
        .args(rest)
        .exec();
    fatal(err);
}

#[cfg(windows)]
fn exec(unity: &Path, project: &Path, rest: &[OsString]) -> ExitCode {
    let status = match std::process::Command::new(unity)
        .arg("-projectPath")
        .arg(project)
        .args(rest)
        .status()
    {
        Ok(s) => s,
        Err(e) => fatal(e),
    };
    std::process::exit(status.code().unwrap_or(1));
}
