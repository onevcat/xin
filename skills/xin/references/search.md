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
  xin search "or:(from:github | from:atlassian) seen:false" --max 20
  xin search --filter-json '{"text":"hello"}' --max 5

Query sugar (not Gmail-compatible):
  from:, to:, subject:, text:, in:, seen:true|false, flagged:true|false
  has:attachment, after:<YYYY-MM-DD>, before:<YYYY-MM-DD>
  -term (NOT), or:(a | b)

Tips:
  - Quote multi-term queries.
  - Use --filter-json for precise server-owned filters (accepts @/path.json).
  - Use --plain for human-friendly output.
```

## JSON Schema

Response: [_schemas/search.json](./_schemas/search.json)
