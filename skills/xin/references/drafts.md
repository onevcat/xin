# drafts

> Drafts operations

## Usage

```
Drafts operations

Usage: xin drafts [OPTIONS] <COMMAND>

Commands:
  list     
  get      
  create   
  update   Metadata-only update that MUST NOT change the draft id
  rewrite  Rewrite a draft's message content by creating a new draft and replacing the old one
  delete   Remove draft(s) from the Drafts mailbox (non-destructive)
  destroy  Permanently destroy draft email(s). Requires global --force
  send     
  help     Print this message or the help of the given subcommand(s)

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

> Usage: xin drafts list [OPTIONS]

```
Usage: xin drafts list [OPTIONS]

Options:
      --json               Output JSON to stdout (default)
      --max <MAX>          
      --page <PAGE>        
      --plain              Output plain text for humans (TSV/block). JSON is the stable contract
      --force              Skip confirmations for destructive commands
      --no-input           Never prompt; fail instead
      --dry-run            Show intended changes without applying
      --account <ACCOUNT>  Choose a configured account (when multiple)
      --verbose            Verbose logging
  -h, --help               Print help
```

### get

> Usage: xin drafts get [OPTIONS] <DRAFT_EMAIL_ID>

```
Usage: xin drafts get [OPTIONS] <DRAFT_EMAIL_ID>

Arguments:
  <DRAFT_EMAIL_ID>  

Options:
      --format <FORMAT>    [default: metadata] [possible values: metadata, full, raw]
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

> Usage: xin drafts create [OPTIONS]

```
Usage: xin drafts create [OPTIONS]

Options:
      --json                   Output JSON to stdout (default)
      --to <TO>                
      --plain                  Output plain text for humans (TSV/block). JSON is the stable contract
      --subject <SUBJECT>      
      --body <BODY>            
      --force                  Skip confirmations for destructive commands
      --body-file <BODY_FILE>  
      --no-input               Never prompt; fail instead
      --body-html <BODY_HTML>  
      --dry-run                Show intended changes without applying
      --account <ACCOUNT>      Choose a configured account (when multiple)
      --cc <CC>                
      --bcc <BCC>              
      --verbose                Verbose logging
      --attach <ATTACH>        
      --identity <IDENTITY>    
  -h, --help                   Print help
```

### update

> Metadata-only update that MUST NOT change the draft id.

```
Metadata-only update that MUST NOT change the draft id.

In v0, JMAP Email properties like subject/from/to/body/attachments are (RFC 8621) immutable, and some servers (e.g. Stalwart) reject changing them via Email/set(update). Use `drafts rewrite` to change message content.

Usage: xin drafts update [OPTIONS] <DRAFT_EMAIL_ID>

Arguments:
  <DRAFT_EMAIL_ID>
          

Options:
      --add <ADD>
          Auto route: mailbox if resolvable, otherwise keyword

      --json
          Output JSON to stdout (default)

      --plain
          Output plain text for humans (TSV/block). JSON is the stable contract

      --remove <REMOVE>
          Auto route: mailbox if resolvable, otherwise keyword

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
          Print help (see a summary with '-h')
```

### rewrite

> Rewrite a draft's message content by creating a new draft and replacing the old one

```
Rewrite a draft's message content by creating a new draft and replacing the old one

Usage: xin drafts rewrite [OPTIONS] <DRAFT_EMAIL_ID>

Arguments:
  <DRAFT_EMAIL_ID>  

Options:
      --destroy-old            Destroy the old draft after rewriting. Requires global --force
      --json                   Output JSON to stdout (default)
      --plain                  Output plain text for humans (TSV/block). JSON is the stable contract
      --to <TO>...             
      --force                  Skip confirmations for destructive commands
      --subject <SUBJECT>      
      --body <BODY>            
      --no-input               Never prompt; fail instead
      --body-file <BODY_FILE>  
      --dry-run                Show intended changes without applying
      --account <ACCOUNT>      Choose a configured account (when multiple)
      --body-html <BODY_HTML>  
      --cc <CC>...             
      --verbose                Verbose logging
      --bcc <BCC>...           
      --attach <ATTACH>        Add attachment(s) by local file path
      --replace-attachments    Replace existing attachments (default: append)
      --clear-attachments      Remove all attachments
      --identity <IDENTITY>    Update From identity for this draft (id or email)
  -h, --help                   Print help
```

### delete

> Remove draft(s) from the Drafts mailbox (non-destructive)

```
Remove draft(s) from the Drafts mailbox (non-destructive)

Usage: xin drafts delete [OPTIONS] [DRAFT_EMAIL_IDS]...

Arguments:
  [DRAFT_EMAIL_IDS]...  

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

### destroy

> Permanently destroy draft email(s). Requires global --force

```
Permanently destroy draft email(s). Requires global --force

Usage: xin drafts destroy [OPTIONS] [DRAFT_EMAIL_IDS]...

Arguments:
  [DRAFT_EMAIL_IDS]...  

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

### send

> Usage: xin drafts send [OPTIONS] <DRAFT_EMAIL_ID>

```
Usage: xin drafts send [OPTIONS] <DRAFT_EMAIL_ID>

Arguments:
  <DRAFT_EMAIL_ID>  

Options:
      --identity <IDENTITY>  
      --json                 Output JSON to stdout (default)
      --plain                Output plain text for humans (TSV/block). JSON is the stable contract
      --force                Skip confirmations for destructive commands
      --no-input             Never prompt; fail instead
      --dry-run              Show intended changes without applying
      --account <ACCOUNT>    Choose a configured account (when multiple)
      --verbose              Verbose logging
  -h, --help                 Print help
```

## JSON Schema

Response: [_schemas/search.json](./_schemas/search.json)
