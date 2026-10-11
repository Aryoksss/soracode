#!/usr/bin/env python3
"""Probe every model in a soracode models file and report which ones answer.

A gateway listing is not a promise. A model can be listed and still fail
upstream with 403, 404, or 503. This asks each configured model for a fixed
reply and prints a verdict, so a broken entry is found before a real run.

Usage:
    probe-models.py [path/to/models.toml] [--timeout SECONDS]

Reads the default models file when no path is given. Exit status is 1 when any
model failed, so it can gate a commit or a release.
"""

import argparse
import json
import os
import sys
import tomllib
import urllib.error
import urllib.request

PROBE = "Reply with exactly: PONG"


def default_models_path() -> str:
    base = os.environ.get("XDG_CONFIG_HOME") or os.path.expanduser("~/.config")
    return os.path.join(base, "soracode", "models.toml")


def load(path: str) -> dict:
    with open(path, "rb") as fh:
        return tomllib.load(fh)


def ask(base_url: str, api_key: str, model: str, timeout: int) -> tuple[str, str]:
    """Return (verdict, detail). Verdict is OK, EMPTY, or an error code."""
    body = json.dumps(
        {
            "model": model,
            "stream": True,
            "max_tokens": 256,
            "messages": [{"role": "user", "content": PROBE}],
        }
    ).encode()
    req = urllib.request.Request(
        base_url.rstrip("/") + "/chat/completions",
        data=body,
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        },
    )
    got = ""
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            for line in resp:
                text = line.decode("utf8", "replace")
                if not text.startswith("data:") or "delta" not in text:
                    continue
                try:
                    delta = json.loads(text[5:])["choices"][0].get("delta", {})
                except Exception:
                    continue
                got += delta.get("content") or ""
                got += delta.get("reasoning_content") or ""
                if "PONG" in got:
                    break
    except urllib.error.HTTPError as exc:
        snippet = exc.read()[:90].decode("utf8", "replace").replace("\n", " ")
        return f"HTTP {exc.code}", snippet
    except Exception as exc:
        return "ERR", str(exc)[:90]

    if "PONG" in got:
        return "OK", ""
    return "EMPTY", repr(got[:70])


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("models", nargs="?", default=default_models_path())
    ap.add_argument("--timeout", type=int, default=60)
    args = ap.parse_args()

    cfg = load(args.models)
    providers = cfg.get("providers", {})
    models = cfg.get("models", {})
    if not models:
        print(f"no [models.*] in {args.models}", file=sys.stderr)
        return 2

    failed = 0
    for name, spec in sorted(models.items()):
        provider = providers.get(spec.get("provider", ""))
        if not provider:
            print(f"  SKIP   {name:<14} unknown provider {spec.get('provider')!r}")
            failed += 1
            continue
        verdict, detail = ask(
            provider["base_url"], provider.get("api_key", ""), spec["model"], args.timeout
        )
        if verdict == "OK":
            print(f"  OK     {name:<14} {spec['model']}")
        else:
            failed += 1
            print(f"  FAIL   {name:<14} {spec['model']}  [{verdict}] {detail}")

    print(f"\n{len(models) - failed}/{len(models)} answered")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
