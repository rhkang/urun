mod config;
mod processes;
mod registry;
mod resolver;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};
use eyre::Report;

#[derive(Parser)]
#[command(
    name = "urun",
    version,
    about = "CLI shim that picks the right Unity editor for each project",
    long_about = "Run `urun <alias> [unity-args…]` to launch Unity for a registered project.",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// register a project
    Add { alias: String, project_path: PathBuf },

    /// unregister a project
    Remove { alias: String },

    /// list all registered projects
    #[command(alias = "ls")]
    List,

    /// print resolved Unity path
    Which { alias: String },

    /// list running Unity editors (alias mapped)
    Ps,

    /// kill running Unity for <alias>
    #[command(alias = "k")]
    Kill { alias: String },

    /// kill all running Unity editors (asks y/n)
    #[command(alias = "ka")]
    KillAll,

    /// launch Unity for a registered project (default: `urun <alias> [unity-args…]`)
    #[command(external_subcommand)]
    Launch(Vec<String>),
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            let _ = e.print();
            return ExitCode::from(if e.use_stderr() { 2 } else { 0 });
        }
    };

    match cli.cmd {
        Cmd::Add {
            alias,
            project_path,
        } => match registry::add(&alias, &project_path) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => fatal_code(e),
        },
        Cmd::Remove { alias } => match registry::remove(&alias) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => fatal_code(e),
        },
        Cmd::List => match registry::list() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => fatal_code(e),
        },
        Cmd::Which { alias } => cmd_which(&alias),
        Cmd::Ps => cmd_ps(),
        Cmd::Kill { alias } => cmd_kill(&alias),
        Cmd::KillAll => cmd_kill_all(),
        Cmd::Launch(args) => match args.split_first() {
            Some((alias, rest)) => cmd_launch(alias, rest),
            None => {
                Cli::command().print_help().ok();
                ExitCode::from(2)
            }
        },
    }
}

fn cmd_ps() -> ExitCode {
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
                .find(|pr| processes::path_matches(p, &pr.path))
                .map(|pr| pr.alias.len())
                .unwrap_or(1),
            None => 1,
        })
        .max()
        .unwrap_or(1);

    for r in &running {
        let alias = match &r.project {
            Some(p) => projects
                .iter()
                .find(|pr| processes::path_matches(p, &pr.path))
                .map(|pr| pr.alias.as_str()),
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

fn cmd_kill(alias: &str) -> ExitCode {
    let project = match registry::lookup(alias) {
        Ok(p) => p,
        Err(e) => return fatal_code(e),
    };
    let target = processes::running().into_iter().find(|r| {
        r.project
            .as_deref()
            .is_some_and(|p| processes::path_matches(p, &project))
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

fn cmd_kill_all() -> ExitCode {
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
                    .find(|pr| processes::path_matches(p, &pr.path))
                    .map(|pr| pr.alias.as_str())
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

fn cmd_which(alias: &str) -> ExitCode {
    match resolver::resolve(alias) {
        Ok(r) => {
            println!("{}", r.unity.display());
            ExitCode::SUCCESS
        }
        Err(e) => fatal_code(e),
    }
}

fn cmd_launch(alias: &str, rest: &[String]) -> ExitCode {
    let resolved = match resolver::resolve(alias) {
        Ok(r) => r,
        Err(e) => return fatal_code(e),
    };
    if is_batchmode(rest) {
        exec_attached(&resolved.unity, &resolved.project, rest)
    } else {
        spawn_detached(&resolved.unity, &resolved.project, rest)
    }
}

fn is_batchmode(args: &[String]) -> bool {
    args.iter().any(|a| a == "-batchmode")
}

fn fatal_code<E>(e: E) -> ExitCode
where
    E: Into<Report>,
{
    print_report(e.into());
    ExitCode::from(1)
}

pub(crate) fn fatal<E>(e: E) -> !
where
    E: Into<Report>,
{
    print_report(e.into());
    std::process::exit(1);
}

fn print_report(report: Report) {
    eprintln!("urun: {}", report);
    for cause in report.chain().skip(1) {
        eprintln!("  caused by: {}", cause);
    }
}

#[cfg(unix)]
fn exec_attached(unity: &Path, project: &Path, rest: &[String]) -> ExitCode {
    use std::os::unix::process::CommandExt;
    let err = std::process::Command::new(unity)
        .arg("-projectPath")
        .arg(project)
        .args(rest)
        .exec();
    fatal(err);
}

#[cfg(windows)]
fn exec_attached(unity: &Path, project: &Path, rest: &[String]) -> ExitCode {
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

#[cfg(unix)]
fn spawn_detached(unity: &Path, project: &Path, rest: &[String]) -> ExitCode {
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;

    let mut cmd = std::process::Command::new(unity);
    cmd.arg("-projectPath")
        .arg(project)
        .args(rest)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    unsafe {
        cmd.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    match cmd.spawn() {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => fatal(e),
    }
}

#[cfg(windows)]
fn spawn_detached(unity: &Path, project: &Path, rest: &[String]) -> ExitCode {
    use std::os::windows::process::CommandExt;
    use std::process::Stdio;

    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

    match std::process::Command::new(unity)
        .arg("-projectPath")
        .arg(project)
        .args(rest)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
        .spawn()
    {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => fatal(e),
    }
}
