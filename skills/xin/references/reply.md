# reply

> Reply to an email by emailId (JMAP Email id)

## Usage

```
Reply to an email by emailId (JMAP Email id)

Usage: xin reply [OPTIONS] <EMAIL_ID>

Arguments:
  <EMAIL_ID>  Original email id (JMAP Email id)

Options:
      --json                   Output JSON to stdout (default)
      --reply-all              Reply-all: include original recipients (To + Cc) in CC
      --plain                  Output plain text for humans (TSV/block). JSON is the stable contract
      --to <TO>...             Override To recipients (otherwise inferred from original From)
      --cc <CC>                Override CC recipients (in addition to reply-all inferred CC)
      --force                  Skip confirmations for destructive commands
      --bcc <BCC>              BCC recipients
      --no-input               Never prompt; fail instead
      --dry-run                Show intended changes without applying
      --subject <SUBJECT>      Subject override. Default: `Re: <original subject>`
      --account <ACCOUNT>      Choose a configured account (when multiple)
      --text <TEXT>            Plain text body. Supports @/path/to/file.txt
      --body-html <BODY_HTML>  HTML body. Supports @/path/to/file.html
      --verbose                Verbose logging
      --attach <ATTACH>        Add attachment(s) by local file path
      --identity <IDENTITY>    Identity to send as (id or email)
  -h, --help                   Print help

Examples:
  xin reply <emailId> --text "Reply text"
  xin reply <emailId> --reply-all --text "Reply all"
  xin reply <emailId> --to other@example.com --text "Custom recipients"

Notes:
  - <emailId> is the JMAP Email id (from `xin search`, `xin messages search`, or `xin inbox next`).
```
