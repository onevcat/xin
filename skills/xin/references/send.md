# send

> Send an email

## Usage

```
Send an email

Usage: xin send [OPTIONS] --to <TO>... --subject <SUBJECT>

Options:
      --json                   Output JSON to stdout (default)
      --to <TO>...             Recipient(s). Can be specified multiple times
      --plain                  Output plain text for humans (TSV/block). JSON is the stable contract
      --subject <SUBJECT>      
      --force                  Skip confirmations for destructive commands
      --text <TEXT>            Plain text body. Supports @/path/to/file.txt
      --body-html <BODY_HTML>  HTML body. Supports @/path/to/file.html
      --no-input               Never prompt; fail instead
      --cc <CC>                
      --dry-run                Show intended changes without applying
      --account <ACCOUNT>      Choose a configured account (when multiple)
      --bcc <BCC>              
      --attach <ATTACH>        Add attachment(s) by local file path
      --verbose                Verbose logging
      --identity <IDENTITY>    Identity to send as (id or email)
  -h, --help                   Print help

Examples:
  xin send --to bob@example.com --subject "Hello" --text "hi"
  xin send --to bob@example.com --subject "Hello" --text @body.txt --attach ./a.pdf
  xin send --to bob@example.com --subject "Hello" --body-html @body.html
  xin send --to bob@example.com --subject "Hello" --text "hi" --identity alice@example.com
```
