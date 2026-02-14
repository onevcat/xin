# attachment

> Download an attachment

## Usage

```
Download an attachment

Usage: xin attachment [OPTIONS] <EMAIL_ID> <BLOB_ID>

Arguments:
  <EMAIL_ID>  
  <BLOB_ID>   

Options:
      --json               Output JSON to stdout (default)
      --out <OUT>          
      --name <NAME>        
      --plain              Output plain text for humans (TSV/block). JSON is the stable contract
      --force              Skip confirmations for destructive commands
      --no-input           Never prompt; fail instead
      --dry-run            Show intended changes without applying
      --account <ACCOUNT>  Choose a configured account (when multiple)
      --verbose            Verbose logging
  -h, --help               Print help

Examples:
  xin attachment <emailId> <blobId>
  xin attachment <emailId> <blobId> --out ./file.bin
  xin --plain attachment <emailId> <blobId>
```

## JSON Schema

Response: (binary download, no JSON response)
