# auth

> 

## Usage

```
Credential helpers

Usage: xin auth [OPTIONS] <COMMAND>

Commands:
  set-token  Store a bearer token for an account (writes tokenFile and updates config)
  help       Print this message or the help of the given subcommand(s)

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

### set-token

```
Store a bearer token for an account (writes tokenFile and updates config)

Usage: xin auth set-token [OPTIONS] <TOKEN>

Arguments:
  <TOKEN>  The bearer token value

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

### help

```
error: unrecognized subcommand '--help'

Usage: xin auth [OPTIONS] <COMMAND>

For more information, try '--help'.
```
