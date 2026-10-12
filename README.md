# soracode

An operator-controlled security agent. One static binary, no runtime to install.

Point it at a target and it plans, runs tools, and keeps what it finds in an
engagement state — hosts, services, vulns, creds, footholds — shared across
every task and saved between runs.

## Install

Build from source (needs [Rust](https://rustup.rs)):

```sh
cargo build --release
install -m755 target/release/soracode ~/.local/bin/soracode
```

Check it is on your `PATH`:

```sh
soracode --version
```

## Quick start

soracode talks to any OpenAI-compatible endpoint. The easiest way to configure
one is to write a starting config, then edit it:

```sh
soracode --init-models     # writes ~/.config/soracode/soracode.jsonc
```

Open that file, set your `baseURL`, and point `apiKey` at an environment
variable:

```jsonc
{
  "model": "9router/gh/gpt-4.1",

  "provider": {
    "9router": {
      "options": {
        "baseURL": "https://9router.example.com/v1",
        "apiKey": "{env:NINEROUTER_API_KEY}"
      },
      "models": {
        "gh/gpt-4.1": { "name": "GPT-4.1" }
      }
    }
  }
}
```

Then export the key and run:

```sh
export NINEROUTER_API_KEY=sk-...
soracode
```

That opens the interactive TUI. For a one-off task without the UI:

```sh
soracode -n -i "scan example.com and report open ports"
```

`soracode --models` shows everything you have configured and which one is
active.

## Configuration

Config lives in `~/.config/soracode/soracode.jsonc`. JSONC means comments and
trailing commas are allowed. A file in the current directory is read too, so
you can keep a per-project config next to your work.

The model file uses the same `provider` / `models` layout as OpenCode, so a
config you already have for another tool reads the way you expect.

### Switch model

```sh
soracode -m 9router/gh/gpt-4.1   # <provider>/<model-id>
soracode -m fast                 # a short name, if you defined one
```

Inside the TUI, `/model` opens a picker.

### Add a model

Add a key under `models`. Its selectable name is `provider/model-id`:

```jsonc
"models": {
  "gh/gpt-4.1":      { "name": "GPT-4.1" },
  "kr/deepseek-3.2": { "name": "DeepSeek 3.2", "temperature": 0.2 }
}
```

### Add a provider

Add another block with its own `options`. Anthropic and Gemini are supported
directly via a `wire` field:

```jsonc
"anthropic": {
  "options": { "baseURL": "https://api.anthropic.com/v1", "wire": "anthropic",
               "apiKey": "{env:ANTHROPIC_API_KEY}" },
  "models": { "claude-sonnet-4-5": { "name": "Claude Sonnet 4.5" } }
}
```

Never write a key into the file — use `{env:VAR}` and export the variable. That
way the config is safe to commit or share.

## MCP servers

soracode can launch any [Model Context Protocol](https://modelcontextprotocol.io)
server over stdio and expose its tools to the agent. Each remote tool shows up
as `mcp_<server>_<tool>` — a Burp Suite MCP server named `burp` turns its
`scan_target` into `mcp_burp_scan_target`:

```jsonc
"mcp": {
  "burp": {
    "command": "java",
    "args": ["-jar", "/opt/burp-mcp.jar", "--stdio"]
  },
  "filesystem": {
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-filesystem", "/srv/evidence"]
  }
}
```

Servers start once per session. One that fails to launch is reported and
skipped, not fatal; `"disabled": true` turns one off without deleting it.

## Tools

The agent runs shell commands and parses the output of the common security
tools into the engagement state, so findings are structured instead of sitting
in scrollback:

| | |
| --- | --- |
| **Recon** | `nmap_parse`, `nuclei_parse`, `gobuster_parse` |
| **Web** | `xss_detect`, `jwt_analyze`, `http` |
| **AD / internal** | `cme_parse`, `bloodhound_parse` |
| **Injection** | `sqlmap_parse` |
| **Engagement** | `state_update`, `state_query`, `attack_path_suggest`, `report_gen` |
| **Filesystem** | `read`, `write`, `edit`, `list`, `glob`, `grep` |
| **Other** | `shell`, `note`, `recall`, `tools`, `subagent`, `skill` |

The engagement state lives in the same SQLite database as the transcript, so it
survives a restart and is shared by every task: a port the scanner finds is
visible to the enumerator immediately.

## Skills

A skill is a Markdown pack that teaches one technique in depth. Packs are
listed in the prompt and loaded only when the agent needs one, so a deep
library never bloats the context window. Six ship built in:

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

## Getting around the TUI

Type an instruction and press `Enter`. Press `/` to see the commands.

| Command | Action |
| --- | --- |
| `/model` | pick a model |
| `/mode <name>` | set autonomy mode (`auto` / `guided` / `free`) |
| `/pause`, `/resume` | stop the agent before its next tool call, and continue |
| `/status` | engagement dashboard |
| `/targets`, `/services` | discovered hosts, services |
| `/vulns`, `/creds`, `/matrix` | findings, credentials, access graph |
| `/tools`, `/sessions` | host tools, recent sessions |
| `/help`, `/clear`, `/quit` | help, clear transcript, exit |

| Key | Action |
| --- | --- |
| `↑` / `↓` | walk back through submitted instructions |
| `PageUp` / `PageDown` | scroll the transcript |
| `Tab` | focus the session sidebar |
| `Ctrl-B` | show or hide the sidebar |
| `Ctrl-T` | show or hide the model's thinking |
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
