# thread

> Thread operations

## Usage

```
Thread operations

Usage: xin thread [OPTIONS] <COMMAND>

Commands:
  get          
  attachments  
  modify       
  archive      
  read         
  unread       
  trash        
  delete       
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

### get

> Usage: xin thread get [OPTIONS] <THREAD_ID>

```
Usage: xin thread get [OPTIONS] <THREAD_ID>

Arguments:
  <THREAD_ID>  

Options:
      --full               
      --json               Output JSON to stdout (default)
      --plain              Output plain text for humans (TSV/block). JSON is the stable contract
      --force              Skip confirmations for destructive commands
      --no-input           Never prompt; fail instead
      --dry-run            Show intended changes without applying
      --account <ACCOUNT>  Choose a configured account (when multiple)
      --verbose            Verbose logging
  -h, --help               Print help

Examples:
  xin thread get <threadId>
  xin thread get <threadId> --full
```

### attachments

> Usage: xin thread attachments [OPTIONS] <THREAD_ID>

```
Usage: xin thread attachments [OPTIONS] <THREAD_ID>

Arguments:
  <THREAD_ID>  

Options:
      --json               Output JSON to stdout (default)
      --plain              Output plain text for humans (TSV/block). JSON is the stable contract
      --force              Skip confirmations for destructive commands
      --no-input           Never prompt; fail instead
      --dry-run            Show intended changes without applying
      --account <ACCOUNT>  Choose a configured account (when multiple)
      --verbose            Verbose logging
  -h, --help               Print help

Examples:
  xin thread attachments <threadId>
```

### modify

> Usage: xin thread modify [OPTIONS] <THREAD_ID>

```
Usage: xin thread modify [OPTIONS] <THREAD_ID>

Arguments:
  <THREAD_ID>  

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

### archive

> Usage: xin thread archive [OPTIONS] <THREAD_ID>

```
Usage: xin thread archive [OPTIONS] <THREAD_ID>

Arguments:
  <THREAD_ID>  

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

### read

> Usage: xin thread read [OPTIONS] <THREAD_ID>

```
Usage: xin thread read [OPTIONS] <THREAD_ID>

Arguments:
  <THREAD_ID>  

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

### unread

> Usage: xin thread unread [OPTIONS] <THREAD_ID>

```
Usage: xin thread unread [OPTIONS] <THREAD_ID>

Arguments:
  <THREAD_ID>  

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

### trash

> Usage: xin thread trash [OPTIONS] <THREAD_ID>

```
Usage: xin thread trash [OPTIONS] <THREAD_ID>

Arguments:
  <THREAD_ID>  

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

### delete

> Usage: xin thread delete [OPTIONS] <THREAD_ID>

```
Usage: xin thread delete [OPTIONS] <THREAD_ID>

Arguments:
  <THREAD_ID>  

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

Response: [_schemas/search-item.json](./_schemas/search-item.json)
