# batch

> Batch operations

## Usage

```
Batch operations

Usage: xin batch [OPTIONS] <COMMAND>

Commands:
  modify  
  delete  
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

### modify

> Usage: xin batch modify [OPTIONS] [EMAIL_IDS]...

```
Usage: xin batch modify [OPTIONS] [EMAIL_IDS]...

Arguments:
  [EMAIL_IDS]...  

Options:
      --add <ADD>
          
      --json
          Output JSON to stdout (default)
      --plain
          Output plain text for humans (TSV/block). JSON is the stable contract
      --remove <REMOVE>
          
      --add-mailbox <ADD_MAILBOX>
          
      --force
          Skip confirmations for destructive commands
      --no-input
          Never prompt; fail instead
      --remove-mailbox <REMOVE_MAILBOX>
          
      --add-keyword <ADD_KEYWORD>
          
      --dry-run
          Show intended changes without applying
      --account <ACCOUNT>
          Choose a configured account (when multiple)
      --remove-keyword <REMOVE_KEYWORD>
          
      --verbose
          Verbose logging
  -h, --help
          Print help
```

### delete

> Usage: xin batch delete [OPTIONS] [EMAIL_IDS]...

```
Usage: xin batch delete [OPTIONS] [EMAIL_IDS]...

Arguments:
  [EMAIL_IDS]...  

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

## JSON Schema

Response: [_schemas/batch-modify.json](./_schemas/batch-modify.json)
