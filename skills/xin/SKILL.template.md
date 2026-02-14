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

## Quick Configuration

```bash
# Initialize config
xin config init

# Set Fastmail API token (Bearer token)
xin auth set-token <TOKEN>
```

## Best Practices

### Output Formats

- **Use `--json` (default)** for scripting and agent use - this is the **stable contract**
- Use `--plain` for human interaction (TSV lists, readable blocks)
- Never parse `--plain` output in automation

### Query Syntax

xin supports a query sugar DSL (NOT Gmail-compatible):

```bash
# Basic operators
from:<email>      # sender
to:<email>        # recipient
subject:<text>   # subject contains
text:<text>       # full-text search

# State
seen:true|false   # $seen keyword
flagged:true|false # $flagged keyword

# Mailbox
in:<mailbox>     # in mailbox (resolves by role, then name)

# Attachments
has:attachment   # has attachments

# Time
after:<YYYY-MM-DD>
before:<YYYY-MM-DD>

# Boolean
-x                # NOT (e.g., -in:Trash)
or:(a | b)        # OR (must be in parentheses)
```

Quote multi-term queries: `xin search "from:github subject:api"`

### Destructive Operations

Always use `--dry-run` first for destructive commands:

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

- High-level overview: [commands.md](./commands.md)

{{REFERENCE_TOC}}

For common workflows and examples, see [common-tasks](./references/common-tasks.md).

For JSON output schema, see [SCHEMA.md](./references/SCHEMA.md).

## Discovering Options

To see available subcommands and flags, run `--help`:

```bash
xin --help
xin search --help
xin inbox --help
xin inbox do --help
```

Each command has detailed help output describing all available flags and options.

## JSON Output and jq

xin outputs stable JSON by default. Common patterns:

```bash
# Extract email IDs
xin search "in:inbox" --max 10 --json | jq -r '.data.items[].emailId'

# Get specific fields
xin search "from:github" --max 5 --json | jq '.data.items[] | {subject, from: .from[0].email}'

# Check for errors
xin search "in:inbox" --json | jq '.error'

# Pagination
xin search "in:inbox" --max 100 --json | jq -r '.meta.nextPage // "none"'
```

## Environment Variables

- `XIN_TOKEN` or `XIN_TOKEN_FILE` - Bearer token
- `XIN_BASE_URL` or `XIN_SESSION_URL` - JMAP endpoint
- `XIN_BASIC_USER` and `XIN_BASIC_PASS` - Basic auth (alternative to Bearer)

## Provider Notes

- `xin url` is **Fastmail-only** - generates Fastmail web URLs
- Other providers will return `xinNotImplemented` for Fastmail-specific features
- xin is RFC-first; provider limitations surface as JMAP errors
