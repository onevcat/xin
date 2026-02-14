# get

> Get a single email

## Usage

```
Get a single email

Usage: xin get [OPTIONS] <EMAIL_ID>

Arguments:
  <EMAIL_ID>  

Options:
      --format <FORMAT>
          [default: metadata] [possible values: metadata, full, raw]
      --json
          Output JSON to stdout (default)
      --max-body-bytes <MAX_BODY_BYTES>
          Max bytes to fetch per body value (only used by --format full). Default: 262144
      --plain
          Output plain text for humans (TSV/block). JSON is the stable contract
      --force
          Skip confirmations for destructive commands
      --headers <HEADERS>
          
      --no-input
          Never prompt; fail instead
      --dry-run
          Show intended changes without applying
      --account <ACCOUNT>
          Choose a configured account (when multiple)
      --verbose
          Verbose logging
  -h, --help
          Print help

Examples:
  xin get <emailId>
  xin get <emailId> --format full
  xin --plain get <emailId> --format full
  xin get <emailId> --headers message-id,in-reply-to

Notes:
  - --format metadata is fast and stable for agents.
  - --format full may include truncation warnings in meta.warnings.
```

## JSON Schema

Response: [_schemas/get.json](./_schemas/get.json)
