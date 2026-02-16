# xin CLI Command Reference

> Generated from xin CLI v0.1.2

xin is an agent-first JMAP CLI for Fastmail. It provides JSON-first output
as the stable contract, with `--plain` for human-friendly output.

## Commands

- [search](./search.md) - Search (thread-like by default)
- [messages](./messages.md) - Per-email search commands
- [get](./get.md) - Get a single email
- [thread](./thread.md) - Thread operations
- [attachment](./attachment.md) - Download an attachment
- [url](./url.md) - Print webmail URL(s) (Fastmail-only)
- [archive](./archive.md) - Archive emails
- [read](./read.md) - Mark emails as read
- [unread](./unread.md) - Mark emails as unread
- [trash](./trash.md) - Move emails to trash
- [batch](./batch.md) - Batch operations
- [inbox](./inbox.md) - Inbox-zero helpers
- [labels](./labels.md) - Labels (mailboxes) operations
- [mailboxes](./mailboxes.md) - Mailboxes operations (alias of labels)
- [identities](./identities.md) - Identities operations
- [send](./send.md) - Send an email
- [reply](./reply.md) - Reply to an email by emailId (JMAP Email id)
- [drafts](./drafts.md) - Drafts operations
- [history](./history.md) - History / changes
- [watch](./watch.md) - Watch for email changes (polling Email/changes; NDJSON stream)
- [config](./config.md) - Config file operations
- [auth](./auth.md) - Credential helpers

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
