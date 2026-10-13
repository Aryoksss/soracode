# soracode

A security agent that works from your terminal. You describe a target, it
plans the work, runs the tools, and remembers everything it learns — so you
stop losing findings in scrollback.

One file to install. Nothing else to set up.

## What it does

You give it a job — "scan example.com", "enumerate the domain controller",
"check this login form". It decides which tools to run, reads their output,
and keeps what matters:

- hosts and open services
- vulnerabilities, by severity
- credentials it finds
- footholds and how they connect

That state is shared across everything you ask it to do and is saved between
runs, so a port the scanner found is ready when you ask about the services on
it a minute later.

Findings stay structured — not buried in a wall of text.

## Install

You only need [Rust](https://rustup.rs) once.

```sh
cargo build --release
install -m755 target/release/soracode ~/.local/bin/soracode
soracode --version
```

## First run

Create a starting config:

```sh
soracode --init-models
```

Open the file it writes (`~/.config/soracode/soracode.jsonc`) and point it at
a model — any service that speaks the OpenAI format works, including a local
one. Put your key in an environment variable and reference it, so the file
itself stays safe to share:

```sh
export PROVIDER_API_KEY=sk-...
soracode
```

If you'd rather skip the UI for one quick job:

```sh
soracode -n -i "scan example.com and report open ports"
```

## Using it

Type what you want in plain language and press `Enter`.

```
> scan 10.10.10.5 and tell me what's exposed
```

The agent shows its plan, the tools it runs, and what it found as it goes.

While it's working you can:

- press `Esc` to stop the current job
- keep typing the next instruction — it waits in line and runs when the
  current job finishes
- press `/` to see the built-in commands

Everything is saved as it happens, so you can close the terminal and pick up
later.

### A few useful commands

| Type this | To |
| --- | --- |
| `/status` | see the whole engagement at a glance |
| `/targets`, `/services` | what hosts and ports you've found |
| `/vulns`, `/creds` | findings and credentials |
| `/model` | switch which model you're using |
| `/mode auto` / `guided` / `free` | how much it stops to ask |
| `/compact` | trim old history when the conversation gets long |
| `/undo` | take back your last message |
| `/export` | save the whole session to a file |
| `/help` | see everything, including keys |
| `/quit` | exit |

### Handy keys

| Key | Does |
| --- | --- |
| `Enter` | send |
| `Shift-Enter` | new line |
| `Up` / `Down` | go back through what you've typed |
| `Esc` | stop the running job |
| `Tab` | open the session list |
| `Ctrl-Y` | copy the last answer to the clipboard |
| `Ctrl-C` | stop the job; press again when idle to quit |

To copy text, just drag and select like anywhere else — soracode never
grabs the mouse drag, so your terminal's normal copy works.

## Sessions

Every job is saved, so an engagement survives a crash or a closed terminal.

```sh
soracode --sessions          # list recent sessions
soracode --resume a0a3cb94   # pick up where you left off
soracode --continue          # resume the newest one
```

Inside the app, press `Tab` to see your sessions.

## How much it decides on its own

| Mode | Behaviour |
| --- | --- |
| `auto` | runs the plan without stopping (the default) |
| `guided` | shows the plan and waits for your go-ahead first |
| `free` | plain assistant, no engagement tracking — good for one-off scripting |

Start in a mode with `--mode`, or change it any time with `/mode`.

## Connecting extra tools (MCP)

soracode can use tools from other programs through
[MCP](https://modelcontextprotocol.io). A server named `burp` turns its
`scan_target` action into `mcp_burp_scan_target` for the agent.

Add a server to your config. For a local program:

```jsonc
"mcp": {
  "filesystem": {
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-filesystem", "/srv/evidence"]
  }
}
```

For one that's already running at a URL — like the Burp Suite MCP extension:

```jsonc
"mcp": {
  "burp": { "url": "http://localhost:9876" }
}
```

If a server won't start, soracode skips it and carries on. Set
`"disabled": true` to turn one off without deleting it.

## Skills

A skill is a short guide that teaches the agent one technique in depth. The
agent sees a one-line summary of each and loads the full text only when a
task calls for it, so a large library never crowds the conversation.

Add your own as `*.md` files in `~/.config/soracode/skills/` or
`<your-project>/.soracode/skills/`. The first line of a file becomes its
summary.

## Standing instructions (memory)

Anything the agent should follow on **every** turn — house rules, a scope
list, your own "never refuse" contract — goes in a memory file. Two names
are read, from the config directory and from your project:

- `~/.config/soracode/SORACODE.md` — your rules, everywhere
- `<your-project>/SORACODE.md` — rules for one engagement
- `AGENTS.md` works too, in either place

Everything found is loaded into the system prompt before the first turn, so
you set it once and forget it.

## License

MIT
