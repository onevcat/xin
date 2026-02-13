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

## Most common agent flows (copy/paste)

> If you’re running from source, prefix each command with:
>
> ```bash
> cargo run --bin xin --
> ```

### 1) List what's currently in my inbox

```bash
# Human-friendly
xin --plain messages search "in:inbox" --max 50

# JSON (stable contract)
xin messages search "in:inbox" --max 50
```

### 2) Archive emails I don't need

```bash
# Archive ONE message
xin inbox do <emailId> archive

# Archive the whole thread containing this message
xin inbox do <emailId> archive --whole-thread

# Archive MANY (example: all inbox items currently returned by search)
xin messages search "in:inbox" --max 200 --json \
  | jq -r '.data.items[].emailId' \
  | xargs -n 50 sh -c 'xin batch modify "$@" --remove inbox --add archive' _
```

### 3) Read one email (details)

```bash
# Full content
xin --plain get <emailId> --format full

# Metadata-only (faster)
xin --plain get <emailId> --format metadata
```

### 4) Reply (send) with a short message

```bash
# Minimal reply (v0): send a new message to the original sender.
# (Threading headers like In-Reply-To are not wired in the CLI yet.)
xin send --to <sender@example.com> --subject "Re: <subject>" --text "XXXX"

# Fastmail-only: generate a web URL for the original message (useful if you want to reply in UI)
xin url <emailId>
```

## Recommended workflow

- **Triage inbox**:
  - `xin messages search "in:inbox" --max 50` → list inbox
  - `xin inbox next` → pick the next email to process (default: 1)
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
