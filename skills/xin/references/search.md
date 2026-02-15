# search

> Search (thread-like by default)

## Usage

```
Search (thread-like by default)

Usage: xin search [OPTIONS] [QUERY]

Arguments:
  [QUERY]  

Options:
      --json
          Output JSON to stdout (default)
      --max <MAX>
          
      --page <PAGE>
          
      --plain
          Output plain text for humans (TSV/block). JSON is the stable contract
      --force
          Skip confirmations for destructive commands
      --oldest
          
      --filter-json <FILTER_JSON>
          
      --no-input
          Never prompt; fail instead
      --collapse-threads <COLLAPSE_THREADS>
          [possible values: true, false]
      --dry-run
          Show intended changes without applying
      --account <ACCOUNT>
          Choose a configured account (when multiple)
      --sort <SORT>
          [default: received-at] [possible values: received-at]
      --verbose
          Verbose logging
  -h, --help
          Print help

Examples:
  xin search "in:inbox" --max 20
  xin search "in:inbox seen:false" --max 20
  xin search "from:github subject:release" --max 10
  xin search "has:attachment after:2026-01-01" --max 20
  xin search "-in:trash" --max 20
  xin search "or:(from:github | from:atlassian) seen:false" --max 20
  xin search --filter-json @filter.json --max 50

Query sugar (not Gmail-compatible):
  from:<text> to:<text> cc:<text> bcc:<text>
  subject:<text> text:<text>
  in:<mailbox> (role/name/id; e.g. inbox, trash, junk, archive)
  seen:true|false flagged:true|false
  has:attachment after:<YYYY-MM-DD> before:<YYYY-MM-DD>
  -term (NOT), or:(a | b)

Tips:
  - Quote multi-term queries.
  - Use --filter-json for precise server-owned filters (accepts @/path.json).
```

## JSON Schema

Response: [_schemas/search.json](./_schemas/search.json)
