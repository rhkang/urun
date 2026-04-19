# uproxy

A project-aware Unity launcher. Register your Unity projects under short
aliases, then invoke Unity against any of them without tracking versions or
install paths — `uproxy` resolves everything at runtime.

```sh
uproxy mygame                                        # open in editor
uproxy mygame -batchmode -quit -executeMethod Build  # headless build
uproxy client-a -batchmode -runTests -testPlatform EditMode -quit
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

`uproxy` collapses all of that into:

```sh
uproxy mygame -batchmode -quit -executeMethod Builder.Build
```

The version comes from the project's own `ProjectVersion.txt`. The editor
path comes from the platform default (or a one-line config override). The
project path comes from an alias you registered once. Everything after the
alias is forwarded to Unity **verbatim** — `uproxy` never parses, rewrites,
or second-guesses Unity's arguments.

### Concretely, this is useful when you

- **Write CI pipelines** that should keep working when a project upgrades
  Unity. Drop the version-sniffing preamble; `uproxy <alias> <args…>` is
  the whole command.
- **Run the same automation across multiple projects** on different Unity
  versions. No per-project shell wrappers.
- **Maintain build scripts shared across machines** where Unity lives in
  different locations. `~/.uproxy/config.toml` is the only per-machine
  knob.
- **Onboard new engineers** to an existing automation setup — `uproxy add
  <alias> <path>` once, then every script in the repo Just Works.

## Install

```sh
cargo install --path .
# or
cargo build --release && cp target/release/uproxy ~/.local/bin/
```

## CLI

```
uproxy <alias> [unity-args…]        launch Unity for a registered project
uproxy add <alias> <project-path>   register a project
uproxy remove <alias>               unregister a project
uproxy list                         list all registered projects
uproxy which <alias>                print resolved Unity.exe path, do not exec
uproxy --version                    print uproxy version
```

Everything after `<alias>` is passed straight to `Unity.exe`, preceded by
`-projectPath <resolved-path>`. `uproxy` adds nothing else.

## Resolution

When `uproxy <alias> [args…]` runs:

1. Look up `<alias>` in `~/.uproxy/projects.toml` → project path.
2. Read version from `<project>/ProjectSettings/ProjectVersion.txt`.
3. Resolve editor root (config override, else platform default).
4. `exec` `<editor-root>/<version>/Editor/Unity.exe -projectPath <project> [args…]`.

Unknown alias, missing `ProjectVersion.txt`, or editor-not-installed all
fail loudly **before** any process is spawned.

## Config

Everything lives under `~/.uproxy/`:

```
~/.uproxy/
├── config.toml      # optional — editor_root override
└── projects.toml    # alias → project path registry
```

`projects.toml` (managed by `uproxy add` / `remove`):

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

- On Unix, `uproxy` `execv()`s Unity — Unity replaces the `uproxy` process.
  No wrapper PID, no signal forwarding layer.
- On Windows, `uproxy` spawns Unity, waits, and forwards the exit code.
- `uproxy` will not launch if the resolved `Unity.exe` doesn't exist. It
  tells you the exact path it expected so you know which editor version
  to install.

## Non-goals

- `uproxy` does **not** download or install Unity editors — use Unity Hub.
- `uproxy` does **not** manage Unity licenses.
- `uproxy` does **not** parse or validate Unity's own arguments.
- `uproxy` does **not** manage multiple Hub install roots.

See [`.claude/Architecture.md`](./.claude/Architecture.md) for the full
design document.
