# config

> 

## Usage

```
Config file operations

Usage: xin config [OPTIONS] <COMMAND>

Commands:
  init         Initialize a minimal config file (fastmail default) if missing
  list         List configured accounts
  set-default  Set the default account
  show         Show config (secrets are never printed)
  help         Print this message or the help of the given subcommand(s)

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

### init

```
Initialize a minimal config file (fastmail default) if missing

Usage: xin config init [OPTIONS]

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

### list

```
List configured accounts

Usage: xin config list [OPTIONS]

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

### set-default

```
Set the default account

Usage: xin config set-default [OPTIONS] <ACCOUNT>

Arguments:
  <ACCOUNT>  

Options:
      --json      Output JSON to stdout (default)
      --plain     Output plain text for humans (TSV/block). JSON is the stable contract
      --force     Skip confirmations for destructive commands
      --no-input  Never prompt; fail instead
      --dry-run   Show intended changes without applying
      --verbose   Verbose logging
  -h, --help      Print help
```

### show

```
Show config (secrets are never printed)

Usage: xin config show [OPTIONS]

Options:
      --effective          Show the merged effective config (CLI/env/config), without secrets
      --json               Output JSON to stdout (default)
      --plain              Output plain text for humans (TSV/block). JSON is the stable contract
      --force              Skip confirmations for destructive commands
      --no-input           Never prompt; fail instead
      --dry-run            Show intended changes without applying
      --account <ACCOUNT>  Choose a configured account (when multiple)
      --verbose            Verbose logging
  -h, --help               Print help
```

### help

```
error: unrecognized subcommand '--help'

Usage: xin config [OPTIONS] <COMMAND>

For more information, try '--help'.
```
