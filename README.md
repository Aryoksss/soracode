# soracode

A lightweight, fully operator-controlled security agent in Rust.

One static binary, no runtime to install. The prompt and the tool layer belong
to this project, so the operator's instruction is the only thing that governs
behavior.

The multi-provider layout follows [Cybermes](https://github.com/Zyrexnn/Cybermes),
which separates a provider (endpoint plus key) from the model running on it.
Soracode keeps that separation in one `models.toml` rather than a `.env`.

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

## Configuration

Highest precedence wins:

1. CLI flags
2. `SORACODE_BASE_URL`, `SORACODE_API_KEY`, `SORACODE_MODEL`, `SORACODE_DEFAULT_MODEL`
3. `--config <file>`
4. `./soracode.toml`, `./soracode.models.toml`
5. `~/.config/soracode/config.toml` or `config.json`, `models.toml`
6. built-in defaults

A minimal config, kept for compatibility:

```toml
[llm]
base_url = "https://9router.aryoks.tech/v1"
api_key  = "sk-..."
model    = "gh/gpt-4.1"

[agent]
workspace = "/home/user/target"
```

## Models

Flat `[llm]` settings hold one endpoint, one key, and one model. That is fine
until you want a second model, and then the endpoint and key have to be
repeated. Named providers and models fix that: a provider is an endpoint plus a
key, and a model points at a provider.

```sh
soracode --init-models      # write ~/.config/soracode/models.toml
soracode --models           # show what is configured and which is active
```

```toml
default_model = "fast"

[providers.router]
base_url = "https://9router.aryoks.tech/v1"
api_key  = "sk-..."

[providers.local]
base_url = "http://127.0.0.1:1234/v1"
api_key  = "lm-studio"          # ignored servers can put anything here

[models.fast]
provider = "router"
model    = "gh/gpt-4.1"

[models.smart]
provider   = "router"
model      = "kr/deepseek-3.2"
max_tokens = 32768
temperature = 0.2               # per-model overrides win over the globals

[models.offline]
provider = "local"
model    = "qwen2.5-coder-32b"
```

Add a model by adding a block. Point it at another provider by changing one
word. Remove it by deleting the block. No code, rebuild, or restart is
involved, because the file is read at startup.

Switching is a single flag:

```sh
soracode -m fast     # name from models.toml
soracode -m smart
soracode -m gh/gpt-4.1   # a raw model string still works
```

The header names the model and where it came from, so a run is never ambiguous:

```
soracode: session 85659c60 | model gh/gpt-4o-mini via <inline> | workspace /w
```

Names are checked against the file, so a misspelling is reported locally rather
than as a confusing `404` from the provider:

```
soracode: unknown model `fst`; configured models: fast, smart, offline.
          Use `literal:fst` to send it verbatim.
```

A name containing `/` or `:` is always treated as a raw model string, and
`literal:` forces that for any name. With no models file, every `--model` value
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

## Tools

| Tool | Purpose |
| --- | --- |
| `shell` | run a command, return stdout+stderr |
| `http` | raw requests, TLS verification off, redirects off by default |
| `read` `write` `edit` `list` `glob` `grep` | filesystem work |
| `note` `recall` | scratch memory re-injected every turn |
| `tools` | list what is installed on this host |
| `subagent` | run several self-contained tasks in parallel |

Subagents get their own context and cannot see the parent conversation, so
their instruction must be self-contained. Depth is capped by
`agent.subagent_max_depth`.

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
| `End` | jump to the newest output and follow it |
| `Ctrl-C` or `Ctrl-Q` | quit |

On exit the TUI prints the session id and the exact command to resume it, so a
run can be picked up again without consulting `--sessions`:

```
soracode: session 593f5ef0 — resume with: soracode --resume 593f5ef0
```

