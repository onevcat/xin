# send

> Send an email

## Usage

```
Send an email

Usage: xin send [OPTIONS] --subject <SUBJECT>

Options:
      --json
          Output JSON to stdout (default)
      --to <TO>...
          Recipient(s). Can be specified multiple times
      --plain
          Output plain text for humans (TSV/block). JSON is the stable contract
      --subject <SUBJECT>
          
      --force
          Skip confirmations for destructive commands
      --text <TEXT>
          Plain text body. Supports @/path/to/file.txt
      --body-html <BODY_HTML>
          HTML body. Supports @/path/to/file.html
      --no-input
          Never prompt; fail instead
      --cc <CC>
          
      --dry-run
          Show intended changes without applying
      --account <ACCOUNT>
          Choose a configured account (when multiple)
      --bcc <BCC>
          
      --attach <ATTACH>
          Add attachment(s) by local file path
      --verbose
          Verbose logging
      --identity <IDENTITY>
          Identity to send as (id or email)
      --reply-to-message-id <REPLY_TO_MESSAGE_ID>
          Reply to a specific email. Sets In-Reply-To and References headers based on the original email's Message-ID
      --reply-all
          Reply-all: auto-populate recipients from the original message. - Original From -> new To - Original To + Cc -> new Cc Requires --reply-to-message-id
      --reply-to <REPLY_TO>
          Custom Reply-To header address
  -h, --help
          Print help

Examples:
  xin send --to bob@example.com --subject "Hello" --text "hi"
  xin send --to bob@example.com --subject "Hello" --text @body.txt --attach ./a.pdf
  xin send --to bob@example.com --subject "Hello" --body-html @body.html
  xin send --to bob@example.com --subject "Hello" --text "hi" --identity alice@example.com

Reply examples:
  xin send --reply-to-message-id "<msg@example.com>" --subject "Re: Hi" --text "Reply text"
  xin send --reply-to-message-id "<msg@example.com>" --reply-all --subject "Re: Hi" --text "Reply all"
  xin send --reply-to-message-id "<msg@example.com>" --reply-to "other@example.com" --subject "Re: Hi" --text "Custom reply-to"
```

## JSON Schema

Response: [_schemas/send.json](./_schemas/send.json)
