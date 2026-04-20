mod config;
mod processes;
mod registry;
mod resolver;

use std::env;
use std::ffi::OsString;
use std::fmt::Display;
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "\
urun — project-aware Unity launcher

USAGE:
    urun <alias> [unity-args…]        launch Unity for a registered project
    urun add <alias> <project-path>   register a project
    urun remove <alias>               unregister a project
    urun list | ls                    list all registered projects
    urun which <alias>                print resolved Unity.exe path
    urun ps                           list running Unity editors (alias mapped)
    urun kill | k <alias>             kill running Unity for <alias>
    urun kill-all | ka                kill all running Unity editors (asks y/n)
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
        Some("list") | Some("ls") => cmd_list(&args),
        Some("which") => cmd_which(&args),
        Some("ps") => cmd_ps(&args),
        Some("kill") | Some("k") => cmd_kill(&args),
        Some("kill-all") | Some("ka") => cmd_kill_all(&args),
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

fn cmd_ps(args: &[OsString]) -> ExitCode {
    if !args.is_empty() {
        eprintln!("usage: urun ps");
        return ExitCode::from(2);
    }
    let projects = match registry::load_projects() {
        Ok(p) => p,
        Err(e) => return fatal_code(e),
    };
    let running = processes::running();
    if running.is_empty() {
        println!("(no Unity processes running)");
        return ExitCode::SUCCESS;
    }

    use std::io::IsTerminal;
    let tty = std::io::stdout().is_terminal();
    let bold = if tty { "\x1b[1;32m" } else { "" };
    let dim = if tty { "\x1b[2m" } else { "" };
    let reset = if tty { "\x1b[0m" } else { "" };

    let alias_width = running
        .iter()
        .map(|r| match &r.project {
            Some(p) => projects
                .iter()
                .find(|(_, pp)| processes::path_matches(p, pp))
                .map(|(a, _)| a.len())
                .unwrap_or(1),
            None => 1,
        })
        .max()
        .unwrap_or(1);

    for r in &running {
        let alias = match &r.project {
            Some(p) => projects
                .iter()
                .find(|(_, pp)| processes::path_matches(p, pp))
                .map(|(a, _)| a.as_str()),
            None => None,
        };
        let project_str = r
            .project
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(no -projectPath)".to_string());
        match alias {
            Some(a) => println!(
                "{bold}* {:<width$}  pid {:<6}  {}{reset}",
                a,
                r.pid,
                project_str,
                width = alias_width,
            ),
            None => println!(
                "{dim}  {:<width$}  pid {:<6}  {}{reset}",
                "-",
                r.pid,
                project_str,
                width = alias_width,
            ),
        }
    }
    ExitCode::SUCCESS
}

fn cmd_kill(args: &[OsString]) -> ExitCode {
    if args.len() != 1 {
        eprintln!("usage: urun kill <alias>");
        return ExitCode::from(2);
    }
    let alias = args[0].to_string_lossy();
    let project = match registry::lookup(&alias) {
        Ok(p) => p,
        Err(e) => return fatal_code(e),
    };
    let target = processes::running().into_iter().find(|r| {
        r.project
            .as_deref()
            .map_or(false, |p| processes::path_matches(p, &project))
    });
    match target {
        Some(r) => match processes::kill_pid(r.pid) {
            Ok(()) => {
                println!("killed {} (pid {})", alias, r.pid);
                ExitCode::SUCCESS
            }
            Err(e) => fatal_code(e),
        },
        None => {
            eprintln!("urun: no Unity process running for alias '{}'", alias);
            ExitCode::from(1)
        }
    }
}

fn cmd_kill_all(args: &[OsString]) -> ExitCode {
    if !args.is_empty() {
        eprintln!("usage: urun kill-all");
        return ExitCode::from(2);
    }
    let running = processes::running();
    if running.is_empty() {
        println!("(no Unity processes running)");
        return ExitCode::SUCCESS;
    }
    let projects = registry::load_projects().unwrap_or_default();

    println!("The following Unity processes will be killed:");
    for r in &running {
        let alias = r
            .project
            .as_deref()
            .and_then(|p| {
                projects
                    .iter()
                    .find(|(_, pp)| processes::path_matches(p, pp.as_path()))
                    .map(|(a, _)| a.as_str())
            })
            .unwrap_or("-");
        let path_str = r
            .project
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(no -projectPath)".to_string());
        println!("  {:<12} pid {:<6}  {}", alias, r.pid, path_str);
    }
    print!("kill all? [y/N] ");
    if std::io::stdout().flush().is_err() {
        return ExitCode::from(1);
    }
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return ExitCode::from(1);
    }
    let ans = input.trim();
    if !matches!(ans, "y" | "Y" | "yes" | "YES") {
        println!("aborted");
        return ExitCode::SUCCESS;
    }
    let mut failed = false;
    for r in &running {
        match processes::kill_pid(r.pid) {
            Ok(()) => println!("killed pid {}", r.pid),
            Err(e) => {
                eprintln!("urun: {}", e);
                failed = true;
            }
        }
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
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
