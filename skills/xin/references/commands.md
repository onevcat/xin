# xin CLI Command Reference

> Generated from xin CLI v0.1.0

xin is an agent-first JMAP CLI for Fastmail. It provides JSON-first output
as the stable contract, with `--plain` for human-friendly output.

**Default output is JSON (stable contract)**; use `--plain` only for humans.

---

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

---

## Commands by Category

### Setup / auth

- [config](./config.md) — config file operations
- [auth](./auth.md) — credential helpers

### Read (search / fetch)

- [search](./search.md) — search threads (collapseThreads default)
- [messages](./messages.md) — per-email search
- [get](./get.md) — fetch one email (metadata/full/raw)
- [thread](./thread.md) — thread get / attachments / modify
- [attachment](./attachment.md) — download an attachment blob
- [url](./url.md) — Fastmail-only web URL helper

### Organize (archive/read/unread/trash/labels)

- [archive](./archive.md) — archive emails
- [read](./read.md) — mark emails as read
- [unread](./unread.md) — mark emails as unread
- [trash](./trash.md) — move emails to trash
- [batch](./batch.md) — batch modify / delete
- [inbox](./inbox.md) — inbox-zero helpers (`next`, `do`)
- [labels](./labels.md) — labels (mailboxes) operations
- [mailboxes](./mailboxes.md) — mailboxes operations (alias of labels)

### Write (send / drafts)

- [identities](./identities.md) — list/get sending identities
- [send](./send.md) — send a message (text/html/attachments)
- [drafts](./drafts.md) — drafts operations

### Automation / sync

- [history](./history.md) — incremental cursor (Email/changes)
- [watch](./watch.md) — polling-based stream (NDJSON)

---

## Related Docs

- Common workflows: [common-tasks.md](./common-tasks.md)
- JSON output schema: [SCHEMA.md](./SCHEMA.md)
