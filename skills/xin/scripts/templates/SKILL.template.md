---
name: xin
description: Use xin CLI to manage JMAP email (Fastmail-first). Covers search, read, send, drafts, labels, and automation.
allowed-tools: Bash(xin:*)
---

# xin CLI

Agent-first JMAP CLI for Fastmail email management.

Generated from xin CLI v{{VERSION}}

## Prerequisites

The `xin` command must be available on PATH. To check:

```bash
xin --version
```

If not built, see: https://github.com/onevcat/xin

## Quick Configuration (Fastmail)

```bash
# Store Fastmail API token (this bootstraps a minimal config if missing)
xin auth set-token fmu1-xxxxx
```

## Best Practices

### Output Formats

- Default output is **stable JSON** (agent-first contract)
- Use `--plain` only for quick human confirmation (not a stability contract)
- Never parse `--plain` output in automation

### Agent pattern: inbox → jq → act

```bash
# Get inbox items (per-email)
xin messages search "in:inbox" --max 200 \
  | jq -r '.data.items[] | [.emailId, (.subject // "")] | @tsv'

# Example: pick only subjects matching "invoice" and archive them
xin messages search "in:inbox" --max 200 \
  | jq -r '.data.items[]
    | select((.subject // "") | test("invoice"; "i"))
    | .emailId' \
  | xargs -n 50 sh -c 'xin batch modify "$@" --remove inbox --add archive' _
```

### Query Syntax

xin supports a query sugar DSL (NOT Gmail-compatible):

```bash
# Mailbox
in:<mailbox>            # resolves by role, then name (e.g. inbox, trash, junk)

# Basic operators
from:<text>
to:<text>
cc:<text>
bcc:<text>
subject:<text>
text:<text>

# State
seen:true|false         # $seen keyword
flagged:true|false      # $flagged keyword

# Attachments + time
has:attachment
after:<YYYY-MM-DD>
before:<YYYY-MM-DD>

# Boolean
-term                   # NOT (e.g., -in:trash)
or:(a | b)              # OR
```

Quote multi-term queries: `xin search "from:github subject:release"`.

### Destructive Operations

Use `--dry-run` first for destructive commands:

```bash
xin batch modify <emailId> --remove inbox --add archive --dry-run
```

### File-based Input

For long content, read from file using `@/path`:

```bash
xin send --to user@example.com --subject "Hello" --text @/tmp/body.txt
```

## Available Commands

{{COMMANDS}}

## Reference Documentation

- High-level overview: [commands.md](references/commands.md)

{{REFERENCE_TOC}}

For common workflows and examples, see [common-tasks](./references/common-tasks.md).

For JSON output schema, see [JSON Schemas](./references/_schemas/index.json).

## Discovering Options

To see available subcommands and flags, run `--help`:

```bash
xin --help
xin search --help
xin inbox --help
xin inbox do --help
```

## Environment Variables

- `XIN_TOKEN` or `XIN_TOKEN_FILE` - Bearer token
- `XIN_BASE_URL` or `XIN_SESSION_URL` - JMAP endpoint
- `XIN_BASIC_USER` and `XIN_BASIC_PASS` - Basic auth (alternative to Bearer)

## Provider Notes

- `xin url` is **Fastmail-only** - generates Fastmail web URLs
- Other providers will return `xinNotImplemented` for Fastmail-specific features
- xin is RFC-first; provider limitations surface as JMAP errors
