# xin JSON Schema (from source)

This document summarizes the JSON shapes **as implemented in the current xin source code**.

Primary source files:
- `src/output.rs` (top-level envelope + meta)
- `src/schema.rs` (email/thread/body/attachments shapes)
- `src/commands/read.rs` (conditional `email.headers` injection)

> This is not meant to replace the canonical contract doc. The canonical, user-facing contract remains: `docs/SCHEMA.md` (copied into this skill as `references/SCHEMA.md`).

---

## 1) Top-level envelope (`src/output.rs`)

All JSON outputs are wrapped in an envelope:

```jsonc
{
  "schemaVersion": "0.1",
  "ok": true,
  "command": "search",
  "account": "fastmail",
  "data": {},
  "error": null,
  "meta": {
    "requestId": "...",
    "nextPage": "...",
    "warnings": ["..."],
    "debug": {"...": "..."}
  }
}
```

Notes:
- `schemaVersion` is hard-coded to `"0.1"` in both `Envelope::ok` and `Envelope::err`.
- `meta.requestId` / `meta.nextPage` / `meta.warnings` are `Option` fields and may be omitted.
- `meta.debug` is explicitly **not stable** (only when debug flags are enabled).

---

## 2) Search item shape (`src/schema.rs::email_summary_item`)

Used by:
- `xin search ...`
- `xin messages search ...`
- `xin inbox next ...` (plus extra convenience fields)

Item:

```jsonc
{
  "threadId": "T...",
  "emailId": "M...",
  "receivedAt": "2026-02-08T06:00:00Z", // or null
  "subject": "...",                    // or null
  "from": [{"name": null, "email": "..."}],
  "to": [{"name": null, "email": "..."}],
  "snippet": "...",                    // preview()
  "hasAttachment": false,
  "mailboxIds": {"<mbxId>": true},
  "keywords": {"$seen": true},
  "unread": false                       // derived: !$seen
}
```

Implementation notes:
- `receivedAt` is derived from JMAP `receivedAt` timestamp, converted to RFC3339; when absent, it becomes `null`.
- `unread` is derived as `!keywords.contains("$seen")`.

---

## 3) Get email data (`src/schema.rs::get_email_data/get_email_full_data`)

Base email metadata object (`email_metadata_object`):

```jsonc
{
  "emailId": "M...",
  "threadId": "T...",
  "receivedAt": "...",
  "subject": "...",
  "from": [],
  "to": [],
  "cc": [],
  "bcc": [],
  "mailboxIds": {"<mbxId>": true},
  "keywords": {"$seen": true},
  "hasAttachment": false,
  "preview": "..."
}
```

`xin get --format metadata` returns:

```jsonc
{
  "email": { /* metadata object */ },
  "body": {"text": null, "html": null},
  "attachments": [],
  "raw": null
}
```

`xin get --format full` returns:

```jsonc
{
  "email": { /* metadata object */ },
  "body": {
    "text": "..." ,
    "html": "..." ,
    "textMeta": {"isTruncated": false, "isEncodingProblem": false},
    "htmlMeta": {"isTruncated": false, "isEncodingProblem": false}
  },
  "attachments": [
    {"emailId": "M...", "blobId": "B...", "name": "a.pdf", "type": "application/pdf", "size": 1234, "disposition": "attachment"}
  ],
  "raw": null
}
```

Warnings:
- Body truncation produces human-readable strings in `meta.warnings[]`.

### 3.1 Optional `email.headers` injection (`src/commands/read.rs`)

When `xin get ... --headers ...` is provided, `read.rs` inserts:

```jsonc
"headers": {
  "message-id": "<...>",
  "received": ["...", "..."],
  "dkim-signature": ["..."],
  "x-custom": "..."
}
```

into the `data.email` object.

---

## 4) Thread get (`src/schema.rs::thread_get_data/thread_get_full_data`)

`xin thread get <threadId>`:

```jsonc
{
  "threadId": "T...",
  "emailIds": ["M1...", "M2..."],
  "emails": [ /* array of email metadata objects */ ]
}
```

With `--full`, `emails[]` becomes an array of full get-like objects:

```jsonc
{
  "threadId": "T...",
  "emailIds": ["M1...", "M2..."],
  "emails": [
    {"email": { /* ... */ }, "body": { /* ... */ }, "attachments": [/*...*/], "raw": null}
  ]
}
```

---

## 5) Thread attachments (`src/schema.rs::thread_attachments_data`)

```jsonc
{
  "threadId": "T...",
  "attachments": [
    {"emailId": "M...", "blobId": "B...", "name": "a.pdf", "type": "application/pdf", "size": 1234, "disposition": "attachment"}
  ]
}
```
