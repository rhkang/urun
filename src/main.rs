mod config;
mod registry;
mod resolver;

use std::env;
use std::ffi::OsString;
use std::fmt::Display;
use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "\
uproxy — project-aware Unity launcher

USAGE:
    uproxy <alias> [unity-args…]        launch Unity for a registered project
    uproxy add <alias> <project-path>   register a project
    uproxy remove <alias>               unregister a project
    uproxy list                         list all registered projects
    uproxy which <alias>                print resolved Unity.exe path
    uproxy --version                    print uproxy version";

fn main() -> ExitCode {
    let mut args: Vec<OsString> = env::args_os().skip(1).collect();
    if args.is_empty() {
        eprintln!("{}", USAGE);
        return ExitCode::from(2);
    }

    let first = args.remove(0);
    match first.to_str() {
        Some("--version") | Some("-V") => {
            println!("uproxy {}", env!("CARGO_PKG_VERSION"));
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
        eprintln!("usage: uproxy add <alias> <project-path>");
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
        eprintln!("usage: uproxy remove <alias>");
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
        eprintln!("usage: uproxy list");
        return ExitCode::from(2);
    }
    match registry::list() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fatal_code(e),
    }
}

fn cmd_which(args: &[OsString]) -> ExitCode {
    if args.len() != 1 {
        eprintln!("usage: uproxy which <alias>");
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
    eprintln!("uproxy: {}", e);
    ExitCode::from(1)
}

pub(crate) fn fatal<E: Display>(e: E) -> ! {
    eprintln!("uproxy: {}", e);
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
