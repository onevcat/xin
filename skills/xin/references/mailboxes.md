# mailboxes

> Mailboxes operations (alias of labels)

## Usage

```
Mailboxes operations (alias of labels)

Usage: xin mailboxes [OPTIONS] <COMMAND>

Commands:
  list    
  get     
  create  
  rename  
  delete  
  modify  
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

### list

> Usage: xin mailboxes list [OPTIONS]

```
Usage: xin mailboxes list [OPTIONS]

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

### get

> Usage: xin mailboxes get [OPTIONS] <MAILBOX>

```
Usage: xin mailboxes get [OPTIONS] <MAILBOX>

Arguments:
  <MAILBOX>  

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

### create

> Usage: xin mailboxes create [OPTIONS] <NAME>

```
Usage: xin mailboxes create [OPTIONS] <NAME>

Arguments:
  <NAME>  

Options:
      --json                   Output JSON to stdout (default)
      --parent <PARENT>        
      --plain                  Output plain text for humans (TSV/block). JSON is the stable contract
      --role <ROLE>            
      --force                  Skip confirmations for destructive commands
      --subscribe <SUBSCRIBE>  [possible values: true, false]
      --no-input               Never prompt; fail instead
      --dry-run                Show intended changes without applying
      --account <ACCOUNT>      Choose a configured account (when multiple)
      --verbose                Verbose logging
  -h, --help                   Print help
```

### rename

> Usage: xin mailboxes rename [OPTIONS] --name <NAME> <MAILBOX_ID>

```
Usage: xin mailboxes rename [OPTIONS] --name <NAME> <MAILBOX_ID>

Arguments:
  <MAILBOX_ID>  

Options:
      --json               Output JSON to stdout (default)
      --name <NAME>        
      --plain              Output plain text for humans (TSV/block). JSON is the stable contract
      --force              Skip confirmations for destructive commands
      --no-input           Never prompt; fail instead
      --dry-run            Show intended changes without applying
      --account <ACCOUNT>  Choose a configured account (when multiple)
      --verbose            Verbose logging
  -h, --help               Print help
```

### delete

> Usage: xin mailboxes delete [OPTIONS] <MAILBOX_ID>

```
Usage: xin mailboxes delete [OPTIONS] <MAILBOX_ID>

Arguments:
  <MAILBOX_ID>  

Options:
      --json               Output JSON to stdout (default)
      --remove-emails      
      --plain              Output plain text for humans (TSV/block). JSON is the stable contract
      --force              Skip confirmations for destructive commands
      --no-input           Never prompt; fail instead
      --dry-run            Show intended changes without applying
      --account <ACCOUNT>  Choose a configured account (when multiple)
      --verbose            Verbose logging
  -h, --help               Print help
```

### modify

> Usage: xin mailboxes modify [OPTIONS] <MAILBOX_ID>

```
Usage: xin mailboxes modify [OPTIONS] <MAILBOX_ID>

Arguments:
  <MAILBOX_ID>  

Options:
      --json                     Output JSON to stdout (default)
      --name <NAME>              
      --parent <PARENT>          
      --plain                    Output plain text for humans (TSV/block). JSON is the stable contract
      --force                    Skip confirmations for destructive commands
      --sort-order <SORT_ORDER>  
      --no-input                 Never prompt; fail instead
      --subscribe <SUBSCRIBE>    [possible values: true, false]
      --dry-run                  Show intended changes without applying
      --account <ACCOUNT>        Choose a configured account (when multiple)
      --verbose                  Verbose logging
  -h, --help                     Print help
```

## JSON Schema

Response: [_schemas/labels-list.json](./_schemas/labels-list.json)
