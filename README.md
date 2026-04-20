# urun

Register your Unity projects under short aliases, then invoke Unity against any of them without tracking versions or install paths — `urun` resolves everything at runtime.

```sh
urun mygame                                        # open in editor
urun mygame -batchmode -quit -executeMethod Build  # headless build
urun client-a -batchmode -runTests -testPlatform EditMode -quit
```

## Why you want this

If you drive Unity from the command line — CI build scripts, test runners,
asset pipeline tools, release automation — you already know the script
everyone ends up writing:

```bash
# The usual mess
UNITY_VERSION=$(grep m_EditorVersion ProjectSettings/ProjectVersion.txt | awk '{print $2}')
UNITY="/c/Program Files/Unity/Hub/Editor/$UNITY_VERSION/Editor/Unity.exe"
"$UNITY" -batchmode -quit -projectPath "$PROJECT_PATH" -executeMethod Builder.Build
```

Every automation repo reinvents this. And every one of them quietly breaks
when:

- A project bumps its Unity version — the script keeps pointing at the old
  editor until someone updates the hardcoded path.
- A teammate installs Unity somewhere other than `C:\Program Files\Unity\Hub\Editor`.
- You juggle multiple client projects on different Unity versions and your
  `PATH` / aliases get out of sync.
- A junior engineer runs the build locally and spends an afternoon figuring
  out why `Unity.exe: command not found`.

`urun` collapses all of that into:

```sh
urun mygame -batchmode -quit -executeMethod Builder.Build
```

The version comes from the project's own `ProjectVersion.txt`. The editor
path comes from the platform default (or a one-line config override). The
project path comes from an alias you registered once. Everything after the
alias is forwarded to Unity **verbatim** — `urun` never parses, rewrites,
or second-guesses Unity's arguments.

### Concretely, this is useful when you

- **Write CI pipelines** that should keep working when a project upgrades
  Unity. Drop the version-sniffing preamble; `urun <alias> <args…>` is
  the whole command.
- **Run the same automation across multiple projects** on different Unity
  versions. No per-project shell wrappers.
- **Maintain build scripts shared across machines** where Unity lives in
  different locations. `~/.local/state/urun/config.toml` is the only per-machine
  knob.
- **Onboard new engineers** to an existing automation setup — `urun add
  <alias> <path>` once, then every script in the repo Just Works.
- **Recover a hung or crashed editor** without hunting through Task Manager.
  Unity occasionally deadlocks on import, wedges on a domain reload, or
  leaves a zombie after a crash — `urun ps` shows every live editor mapped
  back to the alias it's running, and `urun kill <alias>` takes out *that*
  project's editor (not a random `Unity.exe` you hoped was the right one).

## Spotting and killing stuck editors

Unity hangs. It's a fact of life. `urun` keeps a direct link between your
alias and the running editor process, so recovery is one command:

```sh
$ urun ps
* mygame    pid 14820   D:\Projects\MyGame
* client-a  pid 22104   C:\Work\ClientA
  -         pid 31552   E:\Scratch\Prototype     # running, not registered

$ urun kill mygame            # or: urun k mygame
killed mygame (pid 14820)

$ urun kill-all               # or: urun ka — asks y/N first
```

`ps` matches each running `Unity` / `Unity.exe` to a registered alias by
its `-projectPath` argument, so you always know which editor you're about
to kill. Rows with a matching alias are highlighted.

## Install

Requires [Rust / `cargo`](https://rustup.rs).

```sh
cargo install --path .
```

## CLI

```
urun <alias> [unity-args…]        launch Unity for a registered project
urun add <alias> <project-path>   register a project
urun remove <alias>               unregister a project
urun list | ls                    list all registered projects
urun which <alias>                print resolved Unity.exe path, do not exec
urun ps                           list running Unity editors, mapped to aliases
urun kill | k <alias>             kill the Unity editor running <alias>
urun kill-all | ka                kill every running Unity editor (asks y/N)
urun --version                    print urun version
```

Everything after `<alias>` is passed straight to `Unity.exe`, preceded by
`-projectPath <resolved-path>`. `urun` adds nothing else.

## Resolution

When `urun <alias> [args…]` runs:

1. Look up `<alias>` in `~/.local/state/urun/projects.toml` → project path.
2. Read version from `<project>/ProjectSettings/ProjectVersion.txt`.
3. Resolve editor root (config override, else platform default).
4. `exec` `<editor-root>/<version>/Editor/Unity.exe -projectPath <project> [args…]`.

Unknown alias, missing `ProjectVersion.txt`, or editor-not-installed all
fail loudly **before** any process is spawned.

## Config

Everything lives under `~/.local/state/urun/`:

```
~/.local/state/urun/
├── config.toml      # optional — editor_root override
└── projects.toml    # alias → project path registry
```

`projects.toml` (managed by `urun add` / `remove`):

```toml
[projects]
mygame   = "D:/Projects/MyGame"
client-a = "C:/Work/ClientA"
tools    = "/home/dev/UnityTools"
```

`config.toml` (only if your Unity install isn't in the platform default):

```toml
editor_root = "D:/Unity/Hub/Editor"
```

### Platform default editor roots

| Platform | Default path |
|----------|-------------|
| Windows  | `C:\Program Files\Unity\Hub\Editor` |
| macOS    | `/Applications/Unity/Hub/Editor` |
| Linux    | `~/Unity/Hub/Editor` |

## Behaviour notes

- **Interactive launch** (no `-batchmode`): `urun` spawns Unity detached
  from the terminal (new session / detached process) and returns
  immediately. Close the shell or hit Ctrl+C — the editor keeps running.
- **Headless launch** (`-batchmode` in args): `urun` stays attached so
  exit codes, stdout/stderr, and Ctrl+C propagate to Unity. On Unix this
  is an `execv()` — Unity replaces the `urun` process, no wrapper PID.
  On Windows `urun` spawns Unity, waits, and forwards the exit code.
  This is what CI pipelines want.
- `urun` will not launch if the resolved `Unity.exe` doesn't exist. It
  tells you the exact path it expected so you know which editor version
  to install.

## Non-goals

- `urun` does **not** download or install Unity editors — use Unity Hub.
- `urun` does **not** manage Unity licenses.
- `urun` does **not** parse or validate Unity's own arguments.
- `urun` does **not** manage multiple Hub install roots.
