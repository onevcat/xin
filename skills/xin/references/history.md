# history

> 

## Usage

```
History / changes

Usage: xin history [OPTIONS]

Options:
      --json               Output JSON to stdout (default)
      --since <SINCE>      
      --max <MAX>          
      --plain              Output plain text for humans (TSV/block). JSON is the stable contract
      --force              Skip confirmations for destructive commands
      --page <PAGE>        
      --hydrate            When set, also fetch a summary for changed emails (created/updated) via Email/get
      --no-input           Never prompt; fail instead
      --dry-run            Show intended changes without applying
      --account <ACCOUNT>  Choose a configured account (when multiple)
      --verbose            Verbose logging
  -h, --help               Print help

Examples:
  xin history
  xin history --since <state>
  xin history --since <state> --hydrate

Paging:
  - If meta.nextPage is set, continue with: xin history --page <TOKEN>
```
