# messages

> Per-email search commands

## Usage

```
Per-email search commands

Usage: xin messages [OPTIONS] <COMMAND>

Commands:
  search  
  help    Print this message or the help of the given subcommand(s)

Options:
      --json               Output JSON to stdout (default)
      --plain              Output plain text for humans (TSV/block). JSON is the stable contract
      --force              Skip confirmations for destructive commands
      --no-input           Never prompt; fail instead
      --dry-run            Show intended changes without applying
      --account <ACCOUNT>  Choose a configured account (when multiple)
      --verbose            Verbose logging
  -h, --help               Print help
```

## Subcommands

### search

> Usage: xin messages search [OPTIONS] [QUERY]

```
Usage: xin messages search [OPTIONS] [QUERY]

Arguments:
  [QUERY]  

Options:
      --json                       Output JSON to stdout (default)
      --max <MAX>                  
      --page <PAGE>                
      --plain                      Output plain text for humans (TSV/block). JSON is the stable contract
      --filter-json <FILTER_JSON>  
      --force                      Skip confirmations for destructive commands
      --no-input                   Never prompt; fail instead
      --dry-run                    Show intended changes without applying
      --account <ACCOUNT>          Choose a configured account (when multiple)
      --verbose                    Verbose logging
  -h, --help                       Print help

Examples:
  xin messages search "in:inbox" --max 50
  xin messages search "from:github" --max 20
  xin messages search --filter-json @filter.json --max 50

Notes:
  - Default output is stable JSON.
```

## JSON Schema

Response: [_schemas/search.json](./_schemas/search.json)
