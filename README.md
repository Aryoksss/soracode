# soracode

An operator-controlled security agent. One static binary, no runtime to install.

Point it at a target and it plans, runs tools, and keeps what it finds — hosts,
services, vulns, creds, footholds — in an engagement state shared across every
task and saved between runs.

## Install

Needs [Rust](https://rustup.rs).

```sh
cargo build --release
install -m755 target/release/soracode ~/.local/bin/soracode
soracode --version
```

## Setup

soracode talks to any OpenAI-compatible endpoint. Write a starting config, then
fill in your provider:

```sh
soracode --init-models     # writes ~/.config/soracode/soracode.jsonc
```

```jsonc
{
  "model": "<provider>/<model-id>",

  "provider": {
    "<provider>": {
      "options": {
        "baseURL": "https://your-endpoint.example.com/v1",
        "apiKey": "{env:PROVIDER_API_KEY}"
      },
      "models": {
        "<model-id>": { "name": "My Model" }
      }
    }
  }
}
```

Never write a key into the file — use `{env:VAR}` and export it instead, so the
config stays safe to share.

```sh
export PROVIDER_API_KEY=sk-...
soracode
```

That opens the interactive TUI. For a one-off task without the UI:

```sh
soracode -n -i "scan example.com and report open ports"
```

## Providers

OpenAI-compatible is the default and works with any router or local server.
Anthropic and Gemini are supported directly:

```jsonc
"anthropic": {
  "options": { "baseURL": "https://api.anthropic.com/v1", "wire": "anthropic",
               "apiKey": "{env:ANTHROPIC_API_KEY}" },
  "models": { "claude-sonnet-4-5": { "name": "Claude Sonnet 4.5" } }
}
```

Use `"wire": "google"` for Gemini. A local server is the same block with
`"baseURL": "http://127.0.0.1:1234/v1"`.

## Models

Switch with a single flag, or `/model` inside the TUI:

```sh
soracode -m <provider>/<model-id>
```

Add a model by adding a key under `models`; its selectable name is
`provider/model-id`. `soracode --models` lists everything configured.

## MCP servers

soracode speaks [Model Context Protocol](https://modelcontextprotocol.io) and
exposes a server's tools to the agent. A server named `burp` turns its
`scan_target` tool into `mcp_burp_scan_target`.

Two transports are supported. Use `command` for a local stdio server:

```jsonc
"mcp": {
  "filesystem": {
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-filesystem", "/srv/evidence"]
  }
}
```

Use `url` for a remote SSE/HTTP endpoint — for example the Burp Suite MCP
extension, which listens on `http://localhost:9876`:

```jsonc
"mcp": {
  "burp": { "url": "http://localhost:9876" }
}
```

The same block works in `soracode.models.toml`:

```toml
[mcp.burp]
url = "http://localhost:9876"
```

When both `command` and `url` are set, `url` wins. Servers start once per
session. One that fails to launch is skipped, not fatal; `"disabled": true`
turns one off without deleting it.

## Tools

The agent runs shell commands and parses the output of common security tools
into the engagement state, so findings are structured instead of buried in
scrollback.

| | |
| --- | --- |
| **Recon** | `nmap_parse`, `nuclei_parse`, `gobuster_parse` |
| **Web** | `xss_detect`, `jwt_analyze`, `http` |
| **Internal** | `cme_parse`, `bloodhound_parse`, `sqlmap_parse` |
| **Engagement** | `state_update`, `state_query`, `attack_path_suggest`, `report_gen` |
| **Filesystem** | `read`, `write`, `edit`, `list`, `glob`, `grep` |
| **Other** | `shell`, `note`, `recall`, `tools`, `subagent`, `skill` |

The engagement state lives in the same SQLite database as the transcript, so it
survives a restart and is shared by every task: a port the scanner finds is
visible to the enumerator immediately.

## Skills

A skill is a Markdown pack that teaches one technique in depth. Packs are
listed in the prompt and loaded only when needed, so a deep library never
bloats the context window. Six ship built in:

```
ad-attack  cloud-attack  kerberos  linux-privesc  ssrf  web-attack
```

Drop your own `*.md` files into `~/.config/soracode/skills/` or
`<workspace>/.soracode/skills/` and they appear on the next run.

## Sessions

Every instruction, reply, and tool result is saved as it happens, so an
engagement survives a crash or a closed terminal.

```sh
soracode --sessions          # list recent sessions
soracode --resume a0a3cb94   # resume by id
soracode --continue          # resume the newest
```

Delete a session from the TUI sidebar with `d` (or `Delete`).

## Using the TUI

Type an instruction and press `Enter`. Press `/` to see commands. Drag to
select text and copy it as usual — a drag is never captured by the app.

| Command | Action |
| --- | --- |
| `/model` | pick a model |
| `/mode <name>` | autonomy mode (`auto` / `guided` / `free`) |
| `/pause`, `/resume` | stop before the next tool call, and continue |
| `/compact` | trim older turns before the next message, to free context |
| `/undo` | remove the last turn from the session and the view |
| `/export [path]` | save the whole session to a Markdown file |
| `/status` | engagement dashboard |
| `/targets`, `/services` | discovered hosts, services |
| `/vulns`, `/creds`, `/matrix` | findings, credentials, access graph |
| `/tools`, `/sessions` | host tools, recent sessions |
| `/help`, `/clear`, `/quit` | help, clear, exit |

| Key | Action |
| --- | --- |
| `Enter` | send; `Shift-Enter` or `Ctrl-J` for a new line |
| `↑` / `↓` | walk back through submitted instructions |
| `PageUp` / `PageDown` | scroll the transcript |
| mouse wheel | scroll the transcript |
| `Tab` | focus the session sidebar |
| `Ctrl-B` / `Ctrl-T` | toggle sidebar / model thinking |
| `Ctrl-Y` | copy the last reply to the terminal clipboard |
| `Ctrl-L` | clear the transcript |
| `Ctrl-C` | cancel the running turn; quit when idle |
| `Ctrl-Q` | quit |

## Autonomy modes

| Mode | Behaviour |
| --- | --- |
| `auto` | plan, execute and record without stopping (default) |
| `guided` | present a plan and wait for confirmation before state-changing actions |
| `free` | no engagement bookkeeping — a plain assistant for one-off scripting |

Set at startup with `--mode`, or at runtime with `/mode`.

## License

MIT
