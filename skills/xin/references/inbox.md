# inbox

> Inbox-zero helpers

## Usage

```
Inbox-zero helpers

Usage: xin inbox [OPTIONS] <COMMAND>

Commands:
  next  Get the next email to process from Inbox
  do    Apply an action to an email (and optionally its whole thread)
  help  Print this message or the help of the given subcommand(s)

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

### next

> Get the next email to process from Inbox

```
Get the next email to process from Inbox

Usage: xin inbox next [OPTIONS] [QUERY]

Arguments:
  [QUERY]  Additional sugar query appended with AND

Options:
      --all                Include already-seen emails (default: only unread)
      --json               Output JSON to stdout (default)
      --oldest             Oldest-first (default: newest-first)
      --plain              Output plain text for humans (TSV/block). JSON is the stable contract
      --force              Skip confirmations for destructive commands
      --max <MAX>          Max number of items to return (default: 1)
      --no-input           Never prompt; fail instead
      --page <PAGE>        Page token (from meta.nextPage)
      --dry-run            Show intended changes without applying
      --account <ACCOUNT>  Choose a configured account (when multiple)
      --verbose            Verbose logging
  -h, --help               Print help
```

### do

> Apply an action to an email (and optionally its whole thread)

```
Apply an action to an email (and optionally its whole thread)

Usage: xin inbox do [OPTIONS] <EMAIL_ID> <ACTION>

Arguments:
  <EMAIL_ID>  
  <ACTION>    [possible values: archive, trash, read, unread]

Options:
      --json               Output JSON to stdout (default)
      --whole-thread       Apply to the whole thread containing the given email
      --plain              Output plain text for humans (TSV/block). JSON is the stable contract
      --force              Skip confirmations for destructive commands
      --no-input           Never prompt; fail instead
      --dry-run            Show intended changes without applying
      --account <ACCOUNT>  Choose a configured account (when multiple)
      --verbose            Verbose logging
  -h, --help               Print help
```
