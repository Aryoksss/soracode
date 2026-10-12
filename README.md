# soracode

A lightweight, fully operator-controlled security agent in Rust.

One static binary, no runtime to install. The prompt and the tool layer belong
to this project, so the operator's instruction is the only thing that governs
behavior.

The multi-provider layout follows [Cybermes](https://github.com/Zyrexnn/Cybermes),
which separates a provider (endpoint plus key) from the model running on it.
Soracode keeps that separation in one `soracode.jsonc` rather than a `.env`.

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
soracode --continue        # newest session, no id needed

# inspect the exact system prompt
soracode --print-prompt
```

Inside the TUI, slash commands drive the engagement: `/status` shows the
dashboard (hosts, services, vulns, creds, access), `/targets` lists discovered
hosts and their services, `/vulns` lists findings by severity, and `/creds`
lists credentials.

## Configuration

Highest precedence wins:

1. CLI flags
2. `SORACODE_BASE_URL`, `SORACODE_API_KEY`, `SORACODE_MODEL`, `SORACODE_DEFAULT_MODEL`
3. `--config <file>`
4. `./soracode.jsonc`, `./soracode.json`, `./soracode.toml`, `./soracode.models.toml`
5. `~/.config/soracode/soracode.jsonc`/`.json`, `config.toml`, `models.toml`
6. built-in defaults

Config files may be JSON, JSONC (comments and trailing commas allowed), or TOML.

## Models

The model file uses the same `provider`/`models` layout as OpenCode, so a config
you already have for another tool reads the way you expect. Generate a starting
file and list what is configured:

```sh
soracode --init-models      # write ~/.config/soracode/soracode.jsonc
soracode --models           # show what is configured and which is active
```

```jsonc
{
  // the model a plain run uses, as <provider>/<model-id>
  "model": "9router/gh/gpt-4.1",

  "provider": {
    "9router": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "9Router",
      "options": {
        "baseURL": "https://9router.aryoks.tech/v1",
        "apiKey": "{env:NINEROUTER_API_KEY}"   // never write the key in the file
      },
      "models": {
        "gh/gpt-4.1":      { "name": "GPT-4.1",      "limit": { "output": 16384 } },
        "kr/deepseek-3.2": { "name": "DeepSeek 3.2", "limit": { "output": 32768 },
                             "temperature": 0.2 }
      }
    }
  }
}
```

- **Add a model**: add a key under `models`. Its selectable name is
  `provider/model-id`.
- **Switch model**: `-m 9router/gh/gpt-4.1`, the same string that appears in `"model"`.
- **Add a provider**: add another block with its own `options`. A key can be
  read from the environment with `{env:VAR}` instead of being written down.
- **Point at a local server**: same block, `"baseURL": "http://127.0.0.1:1234/v1"`.

A provider speaks one of three wire dialects. OpenAI-compatible is the default
and covers every router and local server; `"wire": "anthropic"` and
`"wire": "google"` select the native Messages and Gemini APIs, which differ in
request body, auth header, and streaming frames. The client translates to and
from those dialects, so a model block, a tool, and the operator's workflow are
identical regardless of which one is in use.

```jsonc
"anthropic": {
  "options": { "baseURL": "https://api.anthropic.com/v1", "wire": "anthropic",
               "apiKey": "{env:ANTHROPIC_API_KEY}" },
  "models": { "claude-sonnet-4-5": { "name": "Claude Sonnet 4.5" } }
},
"gemini": {
  "options": { "baseURL": "https://generativelanguage.googleapis.com/v1beta",
               "wire": "google", "apiKey": "{env:GEMINI_API_KEY}" },
  "models": { "gemini-3-flash": { "name": "Gemini 3 Flash" } }
}
```

`--wire openai|anthropic|google` overrides the dialect for the selected model,
the same way `--base-url` overrides the endpoint.

A TOML layout (`models.toml`) with `[providers.*]` and `[models.*]` is still
read, so an existing file keeps working. Everything below applies to both.

Switching is a single flag:

```sh
soracode -m 9router/gh/gpt-4.1   # provider/model, as in the file
soracode -m fast                 # a short name from models.toml
soracode -m gh/gpt-4.1           # a raw model string still works
```

The header names the model and where it came from, so a run is never ambiguous:

```
soracode: session 85659c60 | model 9router/gh/gpt-4.1 via 9router (openai) | workspace /w
```

Names are checked against the file, so a misspelling is reported locally rather
than as a confusing `404` from the provider:

```
soracode: unknown model `fst`; configured models: fast, smart, offline.
          Use `literal:fst` to send it verbatim.
```

A name that matches a configured model wins, so an OpenCode-style
`provider/model-id` selects it. Any other name containing `/` or `:` is treated
as a raw model string, and `literal:` forces that for any name. With no models
file, every `--model` value
is literal and behavior is unchanged.

### A working catalogue

Not every model a gateway lists is actually served, and a listed model can fail
upstream with `404`, `403`, or `503`. Probe before trusting an entry. The
catalogue below was reached through one 9router endpoint and verified by asking
each model for a fixed reply:

| Name | Model | Notes |
| --- | --- | --- |
| `gpt41` | `gh/gpt-4.1` | default; most obedient, large context |
| `gpt4o` | `gh/gpt-4o` | |
| `gpt4omini` | `gh/gpt-4o-mini` | fast and cheap |
| `deepseek` | `kr/deepseek-3.2` | |
| `qwen-coder` | `kr/qwen3-coder-next` | code |
| `opus` | `kr/claude-sonnet-4.5` | |
| `sonnet46` | `ag/claude-sonnet-4-6` | |
| `opus46` | `ag/claude-opus-4-6-thinking` | reasoning |
| `gemini38` | `ag/gemini-3.8-flash-high` | |
| `gemini38m` | `ag/gemini-3.8-flash` | |
| `gemini3` | `ag/gemini-3-flash` | |
| `gemini31pro` | `ag/gemini-3.1-pro-low` | |
| `gemini-pro` | `ag/gemini-pro-agent` | |
| `gpt55` | `cx/gpt-5.5` | |
| `gpt55r` | `cx/gpt-5.5-review` | |
| `oss120b` | `ollama/gpt-oss:120b` | |
| `oss120b-ag` | `ag/gpt-oss-120b-medium` | reasoning; output arrives in `reasoning_content` |
| `glm` | `cf/@cf/zai-org/glm-4.7-flash` | |
| `llama70b` | `cf/@cf/meta/llama-3.1-70b-instruct-fp8-fast` | sometimes returns JSON instead of the asked-for text |

Models present in the gateway listing but failing when called (`gc/*` answered
`403` without a signed-in Gemini CLI; `cx/gpt-5.4`, `cx/gpt-6-astra`, and
`ag/gemini-3.7-flash` answered `not supported` or `404`; `groq/*` answered
`404`) are left out on purpose, and a failing entry is commented out with the
observed error rather than silently dropped.

A reasoning model whose answer arrives only in `reasoning_content` is read
correctly; it is not treated as an empty reply.

### Reliability and context

A single request is retried automatically on a transport failure or a
retryable status (`408`, `409`, `425`, `429`, and every `5xx`) with an
exponential backoff, so a rate limit or a transient `503` no longer ends the
turn. A failure after the response body has begun streaming is not retried,
because the earlier chunks have already been shown.

```jsonc
"llm": {
  "max_attempts": 4,        // total tries per request, including the first
  "retry_backoff_ms": 500   // first wait; doubles each retry
},
"agent": {
  "max_seconds": 900,       // wall-clock cap for one instruction, 0 = unlimited
  "context_budget": 400000, // approximate conversation chars before compaction
  "system_prompt_file": "/etc/soracode/prompt.md"  // replaces the built-in prompt
}
```

When the conversation exceeds `context_budget`, the oldest whole turns are
elided — never the system prompts, the opening instruction, the most recent
turn, or an assistant tool call separated from its result. The elided span is
replaced by a one-line marker so the model knows context was trimmed.

## Tools

| Tool | Purpose |
| --- | --- |
| `shell` | run a command, return stdout+stderr |
| `http` | raw requests, TLS verification off, redirects off by default |
| `read` `write` `edit` `list` `glob` `grep` | filesystem work |
| `note` `recall` | scratch memory re-injected every turn |
| `tools` | list what is installed on this host |
| `subagent` | run several self-contained tasks in parallel |
| `nmap_parse` | parse nmap XML into hosts and services in the engagement state |
| `nuclei_parse` | parse nuclei JSONL into findings, one per template match |
| `cme_parse` | parse NetExec / CrackMapExec output into hosts, creds and admin access |
| `gobuster_parse` | parse gobuster `dir`/`vhost`/`dns` output into hosts and found paths |
| `sqlmap_parse` | parse sqlmap output into one finding per injectable parameter |
| `bloodhound_parse` | parse BloodHound / SharpHound JSON into hosts, users and graph edges |
| `xss_detect` | decide whether a response reflects a probe unencoded, in script/tag/attribute context |
| `jwt_analyze` | decode a JWT, flag `alg: none` and no-`exp`, and crack a weak HMAC secret |
| `state_update` | record a vuln, credential, foothold or graph edge |
| `state_query` | read the shared engagement state back out |
| `attack_path_suggest` | find shortest attack paths through the recorded graph |
| `report_gen` | render the whole engagement as a Markdown report |
| `skill` | list and load on-demand knowledge packs |

The engagement state (hosts, services, vulns, creds, access, edges) lives in
the same SQLite database as the transcript, so it survives a restart and is
shared by every agent — a port the scanner finds is visible to the enumerator
immediately. Findings, credentials and footholds are deduplicated by content,
so re-parsing the same tool output never multiplies rows.

Subagents get their own context and cannot see the parent conversation, so
their instruction must be self-contained. Depth is capped by
`agent.subagent_max_depth`.

## Skills

A skill is a Markdown knowledge pack that teaches one technique in depth —
Active Directory attacks, Linux privilege escalation, and so on. Packs are
discovered from `~/.config/soracode/skills/` and `<workspace>/.soracode/skills/`,
listed compactly in the prompt, and loaded in full only when the agent calls
the `skill` tool — so a deep library never bloats the context window.

```sh
# two packs ship in .soracode/skills/
.soracode/skills/ad-attack.md
.soracode/skills/linux-privesc.md
```

Drop your own `*.md` files into either directory and they appear on the next
run.

## Knowing the host

Soracode discovers what is actually installed rather than assuming. Every
candidate is resolved by walking the runtime `PATH` the way a shell would, so
tools in `~/.cargo/bin`, `~/go/bin`, and snap directories are found with no
special handling. The result is injected into the prompt, which is why the
agent can say whether `hashcat` exists without running anything.

```sh
soracode --tools            # list what was detected
soracode --tools --rescan   # re-walk PATH and refresh the cache
```

Detection is defensive, because `--version` is not a standard. `ffuf` and `go`
print a usage error to stderr while emitting the version on stdout; `nuclei`
emits ANSI colour codes; and `msfconsole --version` starts the Metasploit
database, so it is reported as present without being probed. Version strings
are scored rather than taken as "the first number", which is what keeps
`ssh -V` reporting `10.2p1` instead of the OpenSSL version it also prints.

Results are cached in `~/.local/share/soracode/tools.json`. A cached entry is
reused only while every recorded path still exists and is executable, so
removing or moving a tool invalidates it. A full scan takes about five seconds;
a cached read is instant.

## Sessions

Transcripts, tool results, and notes live in SQLite at
`~/.local/share/soracode/sessions.db` (WAL mode). Every instruction, reply, and
tool result is persisted as it happens, so an engagement survives a crash or a
closed terminal.

Resuming brings the conversation back, not just the id:

```sh
soracode --resume a0a3cb94   # by id, from --sessions
soracode --continue          # the newest session in this database
```

Only the recent turns are replayed, because a long session would otherwise push
the system prompt past the model's context window. Tool results are not replayed
verbatim, since their request shape is not stored; the assistant text that
followed them is.

### TUI keys

| Key | Action |
| --- | --- |
| `Enter` | submit the instruction |
| `↑` / `↓` | walk back and forward through submitted instructions |
| `Ctrl-U` | clear the input line |
| `PageUp` / `PageDown` | scroll the transcript |
| `Home` / `End` | jump to the top / follow the newest output |
| `Tab` | focus the session sidebar (arrows move, `Enter` resumes, `d` deletes) |
| `Ctrl-B` | show or hide the session sidebar |
| `Ctrl-T` | show or hide the model's thinking |
| `Ctrl-L` | clear the transcript pane |
| `F2` | release mouse capture so drag-select can copy |
| `Ctrl-C` | cancel the running turn; quit when idle |
| `Ctrl-Q` | quit |

The session sidebar accepts the mouse too: click a row to resume it. Press `d`
(or `Delete`) on a focused row to delete that session — the currently open
session is protected, so switch away from it first.

On exit the TUI prints the session id and the exact command to resume it, so a
run can be picked up again without consulting `--sessions`:

```
soracode: session 593f5ef0 — resume with: soracode --resume 593f5ef0
```

### Modes and pause

The agent runs in one of three autonomy modes, set with `--mode` at startup or
`/mode <name>` at runtime:

| Mode | Behaviour |
| --- | --- |
| `auto` | full autonomy — plan, execute and record without stopping (default) |
| `guided` | the agent presents a plan and waits for confirmation before any exploit or state-changing action; read-only recon needs none |
| `free` | no engagement bookkeeping — a plain assistant for one-off scripting |

The mode is prompt-level guidance, not a hard gate: it shapes how the agent
behaves without blocking an action the operator explicitly asked for.

The operator can stop a running engagement at any time with `/pause`. The agent
finishes the turn it is in and stops before the next tool call; `/resume` clears
the pause so the next instruction runs normally.

### Slash commands

| Command | Action |
| --- | --- |
| `/model` | pick a model from the configured list |
| `/mode <name>` | set the autonomy mode (`auto`/`guided`/`free`); no argument shows the current one |
| `/pause` | stop the agent before its next tool call |
| `/resume` | clear the pause |
| `/status` | engagement dashboard: hosts, vulns, creds |
| `/targets` | discovered hosts and services |
| `/vulns` | recorded findings by severity |
| `/creds` | discovered credentials |
| `/tools` | tools detected on this host |
| `/sessions` | list recent sessions |
| `/help` | commands and key bindings |
| `/clear` | clear the transcript |
| `/quit` | exit |


