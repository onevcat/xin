# xin commands (high level)

This is a **workflow-oriented overview** of xin's CLI surface area, with links to the generated per-command reference.

If you only remember one thing: **default output is JSON (stable contract)**; use `--plain` only for humans.

- Common workflows: [references/common-tasks.md](./references/common-tasks.md)
- JSON contract: [references/SCHEMA.md](./references/SCHEMA.md)
- Source-vs-doc schema check: [references/SCHEMA.compare.md](./references/SCHEMA.compare.md)

---

## Setup / auth

- `config` — config file operations → [references/config.md](./references/config.md)
- `auth` — credential helpers → [references/auth.md](./references/auth.md)

---

## Read (search / fetch)

- `search` — search threads (collapseThreads default) → [references/search.md](./references/search.md)
- `messages search` — per-email search → [references/messages.md](./references/messages.md)
- `get` — fetch one email (metadata/full/raw) → [references/get.md](./references/get.md)
- `thread` — thread get / attachments / modify → [references/thread.md](./references/thread.md)
- `attachment` — download an attachment blob → [references/attachment.md](./references/attachment.md)
- `url` — Fastmail-only web URL helper → [references/url.md](./references/url.md)

---

## Organize (archive/read/unread/trash/labels)

- `archive` / `read` / `unread` / `trash` — email-level sugar →
  - [references/archive.md](./references/archive.md)
  - [references/read.md](./references/read.md)
  - [references/unread.md](./references/unread.md)
  - [references/trash.md](./references/trash.md)
- `batch` — batch modify / delete → [references/batch.md](./references/batch.md)
- `inbox` — inbox-zero helpers (`next`, `do`) → [references/inbox.md](./references/inbox.md)
- `labels` / `mailboxes` — mailbox (label) operations →
  - [references/labels.md](./references/labels.md)
  - [references/mailboxes.md](./references/mailboxes.md)

---

## Write (send / drafts)

- `identities` — list/get sending identities → [references/identities.md](./references/identities.md)
- `send` — send a message (text/html/attachments) → [references/send.md](./references/send.md)
- `drafts` — list/get/create/update/rewrite/send/delete/destroy → [references/drafts.md](./references/drafts.md)

---

## Automation / sync

- `history` — incremental cursor (Email/changes) → [references/history.md](./references/history.md)
- `watch` — polling-based stream (NDJSON) → [references/watch.md](./references/watch.md)

---

## Generated reference index

If you want a flat index of generated reference docs, see:
- [references/commands.md](./references/commands.md)
