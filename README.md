# xin（信）

Agent-first **JMAP** CLI (Fastmail-first).

> **Design goal:** `xin` is to JMAP email what `gog gmail` is to Gmail.
> It is **not** a multi-provider wrapper and **not** a replacement for `gog`.

## Quick start (Fastmail)

1) Initialize config:

```bash
xin config init
```

2) Set your Fastmail API token (Bearer token):

```bash
xin auth set-token <TOKEN>
```

3) Search and read:

```bash
# JSON-first (stable contract)
xin search "from:alice seen:false" --max 10
xin get <emailId> --format full

# Human-friendly output
xin --plain search "subject:invoice" --max 5
xin --plain get <emailId> --format full
```

4) Watch for changes (stream):

```bash
# NDJSON stream (agents)
xin watch --checkpoint /tmp/xin.watch.token

# Plain stream (humans)
xin --plain watch --checkpoint /tmp/xin.watch.token
```

## Recommended workflow

- **Triage inbox**:
  - `xin inbox next` → pick the next email
  - `xin inbox do <emailId> <archive|trash|read|unread> [--whole-thread]`
- **Batch organize**:
  - `xin batch modify <emailId>... --add $seen --remove inbox`
  - `xin thread modify <threadId> --add foo --remove $flagged`
- **Write**:
  - `xin send ...`
  - `xin drafts create|rewrite|send ...`
- **Automation**:
  - Use `xin history` / `xin watch` with `--json` (stable) for agents.

## Output formats

- `--json` is the **stable, agent-first contract** (see `docs/SCHEMA.md`).
- `--plain` is **for humans** (TSV for lists, readable blocks for details). It is not a stability contract.

## Provider support

- `xin url` is **Fastmail-only** (generates a Fastmail web URL via `Message-ID`).
- For other providers, xin surfaces `xinNotImplemented` where appropriate.

## Docs

- [Initial spec (overview)](docs/INITIAL.md)
- [CLI contract (full flags/commands)](docs/CLI.md)
- [JSON output schema (agent-first contract)](docs/SCHEMA.md)
- [Implementation notes (RFC-first method plans)](docs/implementation/README.md)

## Development

Run all Rust tests:

```bash
cargo test
```

Run Stalwart feature cases (e2e):

```bash
cargo run --bin xin-feature -- --fresh --case-dir tests/feature/stalwart/cases --all
```
