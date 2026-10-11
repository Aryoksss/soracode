# soracode

A lightweight, fully operator-controlled security agent in Rust.

One static binary, no runtime to install. The prompt and the tool layer belong
to this project, so the operator's instruction is the only thing that governs
behavior.

## Build

```sh
cargo build --release
```

Output is a single ~5 MB binary at `target/release/soracode`.

## Usage

```sh
# one-shot, streams to stdout
soracode -n -i "scan example.com and report open ports"

# interactive TUI
soracode

# target shorthand, equivalent to appending "Target: example.com"
soracode -n -t example.com -i "enumerate the web app"

# list and resume sessions
soracode --sessions
soracode --resume a0a3cb94

# inspect the exact system prompt
soracode --print-prompt
```

## Configuration

Highest precedence wins:

1. CLI flags
2. `SORACODE_BASE_URL`, `SORACODE_API_KEY`, `SORACODE_MODEL`
3. `--config <file>`
4. `./soracode.toml`
5. `~/.config/soracode/config.toml` or `config.json`
6. built-in defaults

A minimal config:

```toml
[llm]
base_url = "https://9router.aryoks.tech/v1"
api_key  = "sk-..."
model    = "gh/gpt-4.1"

[agent]
workspace = "/home/user/target"
```

## Tools

| Tool | Purpose |
| --- | --- |
| `shell` | run a command, return stdout+stderr |
| `http` | raw requests, TLS verification off, redirects off by default |
| `read` `write` `edit` `list` `glob` `grep` | filesystem work |
| `note` `recall` | scratch memory re-injected every turn |
| `subagent` | run several self-contained tasks in parallel |

Subagents get their own context and cannot see the parent conversation, so
their instruction must be self-contained. Depth is capped by
`agent.subagent_max_depth`.

## Sessions

Transcripts, tool results, and notes live in SQLite at
`~/.local/share/soracode/sessions.db` (WAL mode). Every instruction, reply, and
tool result is persisted as it happens, so an engagement survives a crash or a
closed terminal.
