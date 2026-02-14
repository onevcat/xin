# watch

> Watch for email changes (polling Email/changes; NDJSON stream)

## Usage

```
Watch for email changes (polling Email/changes; NDJSON stream)

Usage: xin watch [OPTIONS]

Options:
      --json                       Output JSON to stdout (default)
      --since <SINCE>              Start watching from this state (like history --since)
      --max <MAX>                  Max changes per poll
      --plain                      Output plain text for humans (TSV/block). JSON is the stable contract
      --force                      Skip confirmations for destructive commands
      --page <PAGE>                Page token (source of truth for since/max)
      --hydrate                    When set, also fetch a summary for changed emails (created/updated) via Email/get
      --no-input                   Never prompt; fail instead
      --dry-run                    Show intended changes without applying
      --interval-ms <INTERVAL_MS>  Poll interval in milliseconds when there are no changes [default: 8000]
      --account <ACCOUNT>          Choose a configured account (when multiple)
      --jitter-ms <JITTER_MS>      Random jitter added to interval (milliseconds) [default: 600]
      --checkpoint <CHECKPOINT>    Persist the latest page token to this file (and resume from it if present)
      --verbose                    Verbose logging
      --once                       Exit after reaching a stable point (no more changes)
      --pretty                     Human-friendly output (default is NDJSON for agents)
      --no-envelope                Do not print the final xin JSON envelope line (stream-only)
  -h, --help                       Print help

Examples:
  xin watch --checkpoint /tmp/xin.watch.token
  xin watch --since <state> --once
  xin --plain watch --checkpoint /tmp/xin.watch.token

Notes:
  - Default output is NDJSON stream for agents.
  - Use --no-envelope (or --plain) for stream-only output.
```

## JSON Schema

Response: (NDJSON stream, see SCHEMA.md for details)
