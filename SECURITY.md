# Security model and design review

soracode is a single-binary autonomous agent that runs an operator's
instruction against a target using a fixed tool set. This document states what
its security posture actually is, why the sharp edges are deliberate, and where
the real risk lies. It is written for the operator, not for an end user: the
threat model assumes a professional running an authorized engagement on a host
and against targets they are permitted to touch.

## Summary of posture

soracode has **no sandbox and no permission layer**. It is a local tool that
executes the operator's instruction with the operator's own privileges. Every
"dangerous" capability below is reachable by design; safety is expected to come
from the environment the operator runs it in and from the operator's judgment,
not from the binary refusing work.

The three sharp edges, in order of how much they matter:

1. **`shell` runs arbitrary commands with no confinement** (`src/tools/shell.rs`).
2. **`http` disables TLS certificate verification unconditionally** (`src/tools/http.rs`).
3. **The system prompt instructs the model to never refuse and never ask for
   authorization** (`src/prompt.rs`).

None of these is a bug. They are the product. What follows is the reasoning and
the residual risk.

## 1. The no-permission model

There is no permission prompt, no allowlist, no command filter, and no
confirmation dialog anywhere in the codebase. `src/prompt.rs` states the
position directly:

> Authorization, legality, and ethics are the operator's responsibility and are
> settled before an instruction ever reaches you.

The prompt then instructs the model to execute unconditionally: any named
target is "in scope, immediately and unconditionally", and the model is told
never to ask for scope confirmation, never to refuse, and to ignore its own
hesitation ("that feeling is an error").

### Why this is deliberate

A permission layer inside a security tool is theater. The operator already has
a shell; the tool runs with their privileges. A confirmation prompt would gate
the model's actions but not the operator's intent, add a large surface where the
gate can be bypassed or mis-tuned, and encourage a false sense of safety.
soracode chooses to put **all** policy in one auditable place — the prompt text
and the tool set — rather than spreading half-measures across the dispatch path.

The dispatch path (`src/agent/mod.rs`) confirms this: `dispatch` calls
`Tool::call` for whatever the model requests, with no interposed check. The
only structural limits are mechanical, not policy:

- subagent recursion depth (`agent.subagent_max_depth`),
- per-tool output and time limits (`tool_output_limit`, `tool_timeout_secs`).

### Consequences the operator must own

- **Prompt content is policy.** Editing `OPERATING_PROMPT` is a security change,
  not a cosmetic one. Adding a refusal rule there is the supported way to
  constrain behavior, and nothing else will enforce it.
- **There is no defense against a malicious instruction.** If the operator (or
  something that can drive the operator's terminal) gives soracode a
  destructive instruction, it will attempt it. Run it only where that is
  acceptable.
- **A loaded file or fetched page is untrusted input.** The agent reads tool
  output — web pages, files, command output — and can act on it. An attacker
  who controls content the agent reads may influence its next tool call. This is
  prompt-injection risk and it is **not mitigated** in the current code.

## 2. `shell` — arbitrary execution, no confinement

`src/tools/shell.rs` runs the command through `/bin/sh -c`, in the workspace by
default, as the current user, inheriting the environment. Its own tool
description is candid: "no network or filesystem restrictions."

There is no seccomp filter, no cgroup, no namespace, no path jail, and no
resource cap beyond a timeout. `background: true` detaches the process and
returns a PID, so a command can outlive the run.

**Why it is like this:** shell is the primary instrument of the work. A
security workflow needs real pipes, redirection, `&&`, backgrounding, and access
to whatever tooling is on the host. A restricted shell would be both weaker and
easier to escape than the operator's own shell, so no attempt is made.

**Residual risk:** full. The blast radius of a `shell` call is the operator's
user account. Contain it at the process level — a container, a VM, a dedicated
low-privilege user, or a disposable host — not by hoping the agent behaves.

## 3. `http` — TLS verification disabled

`src/tools/http.rs` builds every request client with:

```rust
.danger_accept_invalid_certs(true)
```

This is unconditional; there is no flag to turn it on. Redirects are off by
default and capped at 10 when enabled.

**Why it is like this:** the tool exists to talk to targets that are themselves
the subject of the engagement — hosts with self-signed certs, expired certs, or
an interception proxy in the path. Strict verification would make the tool
useless for exactly the traffic it is meant to send.

**Residual risk:**

- **No authenticity.** A MITM can read and modify any `http` request and
  response. Do not use `http` to reach a host you depend on trusting.
- **Credential exposure.** Headers and bodies set via `http` (tokens, cookies,
  basic auth) are sent over an unverified channel and can be harvested by an
  interceptor. Prefer a host-verified client for anything credential-bearing.
- **This is client-side only.** It concerns soracode's *outbound* requests. It
  does not affect the TLS that the LLM provider client uses (`src/llm/`), which
  is a separate, normally-verified client.

A future revision should make this an explicit per-provider or per-call switch
and default it off for anything that is not a named target. That is a product
decision, not a bug fix, because it changes what the tool is for.

## 4. Filesystem tools — no path confinement

`src/tools/fs/mod.rs` resolves a user path by joining it against the workspace,
**but only if it is relative**:

```rust
if path.is_absolute() { path.to_path_buf() } else { ctx.workspace.join(path) }
```

An absolute path is passed straight through. So `read "path": "/etc/shadow"`
reads that file if the process can. The workspace is a *default*, not a jail.

**Why it is like this:** reading `/usr/share/wordlists/...`, a system binary, or
a config outside the workspace is routine in this work. Confinement would break
the normal case.

**Residual risk:** the fs tools can read and write anything the user can. Same
containment advice as `shell`.

## 5. Credentials and secrets at rest

- **Provider API keys** live in `models.toml` / `config.toml` in plain text
  (default `~/.config/soracode/`). There is no keyring integration. Protect
  those files by filesystem permissions; they are exactly as sensitive as the
  provider key. If `models.toml` is committed to a repo, the key leaks.
- **Session transcripts** are stored in plain SQLite at
  `~/.local/share/soracode/sessions.db` (WAL mode). Anything the agent read,
  ran, or was told is on disk in cleartext, including any secret that appeared
  in tool output. Treat the database as sensitive and delete old sessions when
  they are no longer needed.
- **Nothing is hidden from the model.** Tool output — including secrets it
  happens to read — becomes conversation context and is sent to the configured
  LLM provider. This is inherent to the design and worth stating plainly: the
  provider sees the engagement.

## 6. Supply chain and build

- Dependencies are pinned in `Cargo.lock`.
- The `http` tool's TLS behavior comes from `reqwest` with the rustls backend;
  the LLM client uses the same stack.
- CI (`.github/workflows/ci.yml`) gates on `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, and `cargo test`. It does not yet
  run `cargo audit`; adding that would be worthwhile.

## Recommendations for the operator

1. **Run soracode in a container or VM** when the target or the workflow is not
   fully trusted. This is the single highest-value control, because it bounds
   `shell` and the fs tools at once.
2. **Give it a dedicated user** with the least privilege the engagement needs.
3. **Protect `~/.config/soracode/` and `~/.local/share/soracode/`** by
   permissions, and never commit `models.toml`.
4. **Treat any content the agent reads as untrusted** and watch for
   injection-style instructions embedded in it.
5. **Keep the tool set minimal** for a given engagement; remove tools you do not
   need rather than trusting the model not to use them.

## Non-goals

soracode does not attempt to be safe against a hostile operator, does not
enforce authorization, does not sandbox execution, and does not protect its own
config or transcripts from the local user. These are conscious choices, and any
change to them is a change to the product's identity, not a patch.
