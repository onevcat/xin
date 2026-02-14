# xin CLI Command Reference

> Generated from xin CLI v0.1.0

xin is an agent-first JMAP CLI for Fastmail. It provides JSON-first output
as the stable contract, with `--plain` for human-friendly output.

## Commands

- [search](./search.md) - 
- [messages](./messages.md) - 
- [get](./get.md) - 
- [thread](./thread.md) - 
- [attachment](./attachment.md) - 
- [url](./url.md) - 
- [archive](./archive.md) - 
- [read](./read.md) - 
- [unread](./unread.md) - 
- [trash](./trash.md) - 
- [batch](./batch.md) - 
- [inbox](./inbox.md) - 
- [labels](./labels.md) - 
- [mailboxes](./mailboxes.md) - 
- [identities](./identities.md) - 
- [send](./send.md) - 
- [drafts](./drafts.md) - 
- [history](./history.md) - 
- [watch](./watch.md) - 
- [config](./config.md) - 
- [auth](./auth.md) - 

## Quick Reference

```bash
# Get help for any command
xin <command> --help
xin <command> <subcommand> --help

# JSON is the stable contract (default)
xin search "from:alice seen:false" --max 10

# --plain is for humans (not a stability contract)
xin --plain search "subject:invoice" --max 5
```
