# xin（信）

`xin` is an **agent-first** command line tool for **JMAP** email (Fastmail-first).

- Default output is **stable JSON** (no flag needed).

> **Design goal:** `xin` is to JMAP email what `gog gmail` is to Gmail.
> It is **not** a multi-provider wrapper and **not** a replacement for `gog`.

## Install (recommended)

Homebrew (macOS / Linux):

```bash
brew install onevcat/tap/xin
```

Minimal demo (Fastmail):

```bash
# This will bootstrap a minimal config automatically if missing.
xin auth set-token fmu1-xxxxx   # Fastmail API token

xin search --max 1
# or: xin inbox next
```

## Quick start (Fastmail)

1) Set your Fastmail API token (Bearer token):

```bash
# This command will bootstrap a minimal config automatically if missing.
# example token format: fmu1-xxxxx
xin auth set-token fmu1-xxxxx
```

2) Search and read (JSON by default):

```bash
xin search "from:alice seen:false" --max 10
xin get <emailId> --format full
```

3) Watch for changes (stream; default is NDJSON for agents):

```bash
xin watch --checkpoint /tmp/xin.watch.token
```

## Agent recipes (copy/paste)

> If you’re running from source, prefix each command with:
>
> ```bash
> cargo run --bin xin --
> ```

### 1) Get inbox → extract subjects → filter with jq

```bash
# 1) Get current inbox (per-email)
xin messages search "in:inbox" --max 200 \
  > /tmp/xin.inbox.json

# 2) Extract subjects (and emailId so an agent can act on it)
jq -r '.data.items[] | [.emailId, (.subject // "")] | @tsv' /tmp/xin.inbox.json

# 3) Filter by subject keyword (case-insensitive)
jq -r '.data.items[]
  | select((.subject // "") | test("invoice"; "i"))
  | {emailId, subject, from: (.from[0].email // null)}'
  /tmp/xin.inbox.json
```

### 2) Archive emails I don’t need (batch)

```bash
# Archive MANY (example: all inbox items currently returned by search)
xin messages search "in:inbox" --max 200 \
  | jq -r '.data.items[].emailId' \
  | xargs -n 50 sh -c 'xin batch modify "$@" --remove inbox --add archive' _
```

### 3) Inbox-zero helper

```bash
# Pick the next email to process (default: unread-only)
xin inbox next

# Apply an action (optionally for the whole thread)
xin inbox do <emailId> archive
xin inbox do <emailId> archive --whole-thread
```

## Query syntax (quick)

`xin search` uses a small sugar DSL (not Gmail-compatible). Common examples:

```bash
xin search "in:inbox" --max 20
xin search "in:inbox seen:false" --max 20
xin search "from:github subject:release" --max 10
xin search "has:attachment after:2026-01-01" --max 20
xin search "-in:trash" --max 20
xin search "or:(from:github | from:atlassian) seen:false" --max 20
```

Full syntax and rules: see [docs/CLI.md](docs/CLI.md).

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
  - Use `xin history` / `xin watch` (JSON/NDJSON by default).

## Human-friendly output (`--plain`)

If you want a simpler, human-friendly view (e.g. to confirm a list before acting), add `--plain`.
It’s **not** a stability contract; agents should keep using the default JSON output.

```bash
xin --plain search "in:inbox" --max 20
xin --plain get <emailId> --format full
```

## Output formats

- Default output is **JSON** (stable, agent-first contract; see `docs/SCHEMA.md`).
- `--plain` is **for humans** (TSV / readable blocks). Don’t parse it in automation.

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
