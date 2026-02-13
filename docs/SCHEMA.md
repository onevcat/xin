# xin JSON Output Schema (v0)

This document defines the **stable, agent-first JSON contract** for xin.

Principles:

- **RFC-first**: xin sends standard JMAP requests (RFC 8620/8621). Provider limitations are surfaced as returned errors.
- **Stable**: xin output is designed to be parsed by agents. Additive changes only in v0.x.
- **Transparent**: xin may include the raw JMAP method error payload when a request fails.

---

## 0) Versioning

Every JSON output MUST include:

- `schemaVersion`: string (e.g. `"0.1"`)

If a breaking change is required, bump the major schema version.

---

## 1) Top-level envelope

All commands that support `--json` MUST output a single object:

```json
{
  "schemaVersion": "0.1",
  "ok": true,
  "command": "search",
  "account": "fastmail",
  "data": {},
  "error": null,
  "meta": {}
}
```

Fields:

- `ok`: boolean
- `command`: string (the invoked command, e.g. `search`, `get`, `labels.list`)
- `account`: string|null (account name from config; null if not applicable)
- `data`: object|null (command-specific payload)
- `error`: object|null (see Error schema)
- `meta`: object (paging, timings, etc.)

### 1.1 meta

Common optional fields:

- `requestId`: string|null (xin-generated)
- `nextPage`: string|null (opaque cursor for `--page`)
- `warnings`: string[]|null
- `debug`: object|null
  - Only present when `--verbose` (or debug env flags) are enabled.
  - **Not stable**: intended for humans/tests; agents should not depend on its shape.
  - Example: body truncation warnings for `get --format full` when `maxBodyValueBytes` is exceeded.

---

## 2) Error schema

When `ok=false`, `error` MUST be non-null.

```json
{
  "kind": "jmapMethodError",
  "message": "Email/set failed",
  "http": { "status": 400 },
  "jmap": {
    "type": "invalidProperties",
    "description": "...",
    "method": "Email/set",
    "details": {}
  }
}
```

Fields:

- `kind`: one of
  - `"xinUsageError"` (invalid CLI args)
  - `"xinParseError"` (DSL / JSON parsing)
  - `"xinConfigError"` (missing account, missing token env, etc.)
  - `"xinNotImplemented"` (command surface exists but not implemented yet)
  - `"httpError"` (non-2xx outside JMAP method errors; includes problem+json if any)
  - `"jmapRequestError"` (request-level JMAP error)
  - `"jmapMethodError"` (method-level JMAP error)
- `message`: human-readable summary

Optional:

- `http.status`: number
- `http.problem`: object|null (RFC7807 problem details if present)
- `jmap`:
  - `type`: string (JMAP error type, e.g. `notFound`, `forbidden`, `invalidProperties`, `blobNotFound`)
  - `description`: string|null
  - `method`: string|null (e.g. `Email/query`, `Mailbox/set`)
  - `details`: object|null (raw JMAP error payload, pass-through)

---

## 3) Common data types

### 3.1 Id

- `Id`: string (JMAP Id; base64url-safe)

### 3.2 Keyword map

- `keywords`: object map of `keyword -> true`
  - Example: `{ "$seen": true, "$flagged": true }`

### 3.3 Address

```json
{ "name": "Alice", "email": "alice@example.com" }
```

- `name`: string|null
- `email`: string

---

## 4) Read outputs

### 4.1 search (thread-like results)

`xin search ...` returns (default `collapseThreads=true`):

```json
{
  "items": [
    {
      "threadId": "T...",
      "emailId": "M...",
      "receivedAt": "2026-02-08T06:00:00Z",
      "subject": "...",
      "from": [{"name":null,"email":"..."}],
      "to": [{"name":null,"email":"..."}],
      "snippet": "...",
      "hasAttachment": false,
      "mailboxIds": {"<mailboxId>": true},
      "keywords": {"$seen": true},
      "unread": false
    }
  ]
}
```

Fields:

- `items[]`:
  - `threadId`: Id
  - `emailId`: Id (representative email for the thread)
  - `receivedAt`: RFC3339 string (UTC)
  - `subject`: string|null
  - `from[]`: Address[]|null
  - `to[]`: Address[]|null
  - `snippet`: string|null (server preview if available)
  - `hasAttachment`: boolean
  - `mailboxIds`: object map `mailboxId -> true`
  - `keywords`: keyword map
  - `unread`: boolean (derived: absence of `$seen`)

### 4.2 messages search (per-email)

Same as `search.items[]` but without implying threading; still includes `threadId` and `emailId`.

### 4.3 inbox next

`xin inbox next ...` is a thin wrapper around `messages search`.

It returns the same `data.items[]` shape, plus two conveniences:

- `data.item`: the first item in `data.items` (or `null`).
- `data.query`: how the query was compiled.

```json
{
  "items": [ { "emailId": "M...", "threadId": "T..." } ],
  "item": { "emailId": "M...", "threadId": "T..." },
  "query": {
    "compiled": "in:inbox seen:false subject:foo",
    "collapseThreads": false,
    "oldest": false,
    "all": false
  }
}
```

### 4.4 inbox do

`xin inbox do <emailId> <action> ...` reuses the underlying sugar outputs (`archive`, `trash`, `read`, `unread`).

- Envelope `command` is `inbox.do`.
- `data` matches the target action’s schema in §6 (organize outputs).

### 4.5 get (single email)

`xin get <emailId> --format metadata|full|raw`

- `metadata`: headers + summary + structure pointers
- `full`: includes best-effort decoded body
- `raw`: includes raw JMAP Email object

Proposed payload:

```json
{
  "email": {
    "emailId": "M...",
    "threadId": "T...",
    "receivedAt": "...",
    "subject": "...",
    "from": [],
    "to": [],
    "cc": [],
    "bcc": [],
    "mailboxIds": {},
    "keywords": {},
    "hasAttachment": false,
    "preview": "...",

    "headers": {
      "message-id": "<...>",
      "references": ["<...>", "<...>"],
      "received": ["from ...", "from ..."],
      "dkim-signature": ["v=1; ..."],
      "x-custom": "..."
    }
  },
  "body": {
    "text": "...",
    "html": "...",
    "textMeta": {"isTruncated": true, "isEncodingProblem": false},
    "htmlMeta": {"isTruncated": false, "isEncodingProblem": false}
  },
  "attachments": [
    { "emailId": "M...", "blobId": "B...", "name": "a.pdf", "type": "application/pdf", "size": 1234, "disposition": "attachment" }
  ],
  "raw": null
}
```

Notes:
- `raw` is non-null only for `--format raw`.
- `email.headers` is present only when `xin get ... --headers ...` is provided.
  - It is a parsed dictionary keyed by **normalized lowercase** header names.
  - Values are scalars for singleton headers, and arrays for headers that may repeat (e.g. `received`, `dkim-signature`, `authentication-results`, `resent-*`).

### 4.6 thread get

`xin thread get <threadId> [--full]`:

Default (metadata):

```json
{
  "threadId": "T...",
  "emailIds": ["M1...","M2..."],
  "emails": [ /* metadata email objects (same shape as get.email) */ ]
}
```

When `--full` is set, `emails[]` items use the same shape as `xin get --format full`:

```json
{
  "threadId": "T...",
  "emailIds": ["M1...","M2..."],
  "emails": [
    { "email": { /* ... */ }, "body": { /* ... */ }, "attachments": [/*...*/], "raw": null }
  ]
}
```

---

## 5) Mailboxes (labels/mailboxes)

### 5.1 labels list / mailboxes list

```json
{
  "mailboxes": [
    {
      "id": "...",
      "name": "Inbox",
      "role": "inbox",
      "parentId": null,
      "sortOrder": 0,
      "totalEmails": 123,
      "unreadEmails": 4,
      "totalThreads": 100,
      "unreadThreads": 3,
      "isSubscribed": true
    }
  ]
}
```

### 5.2 labels get

Returns a single `mailbox` object with the same fields.

### 5.3 labels create/modify/delete

For Mailbox/set-based commands, xin returns a summary of what the server reported:

```json
{
  "created": [ { "id": "mbx1", "name": "A" } ],
  "updated": [ { "id": "mbx2" } ],
  "destroyed": [ "mbx3" ]
}
```

Notes:
- `labels create`: returns `created[]` with at least `{id,name}`.
- `labels rename` / `labels modify`: return `updated[]` with `{id}`.
- `labels delete`: returns `destroyed[]` with ids.
- Mailbox fields like counts or `isSubscribed` may be `null` if the server does not provide them (xin keeps the shape stable).

---

## 6) Organize outputs

For `archive/read/unread/trash` and modify commands, xin returns:

```json
{
  "appliedTo": {
    "emailIds": ["M..."]
  },
  "changes": {
    "mailboxIds": {"added": ["..."], "removed": ["..."]},
    "keywords": {"added": ["$seen"], "removed": []}
  },
  "dryRun": false
}
```

For delete commands, xin returns:

```json
{
  "appliedTo": {
    "emailIds": ["M..."]
  },
  "deleted": ["M..."],
  "dryRun": false
}
```

For thread-level operations (`xin thread ...`), include:

- `appliedTo.threadId`
- `appliedTo.emailIds` (expanded)

---

## 7) Write outputs

### 7.1 identities

- `identities list`:

```json
{ "identities": [ { "id": "I...", "name": "...", "email": "me@example.com" } ] }
```

- `identities get`:

```json
{ "identity": { "id": "I...", "name": "...", "email": "me@example.com" } }
```

### 7.2 send

```json
{
  "draft": { "emailId": "M...", "threadId": "T..." },
  "submission": { "id": "S...", "sendAt": null },
  "uploaded": [ { "blobId": "B...", "type": "...", "size": 123 } ]
}
```

### 7.3 drafts

All `xin drafts ...` commands use the same envelope; their `data` shapes are:

- `drafts list`:

```json
{ "items": [ /* same item shape as SCHEMA.md §4.1 */ ] }
```

- `drafts get`:

```json
{ "draft": { /* same shape as SCHEMA.md §4.3 get.email */ }, "body": {"text": "...", "html": "..."}, "attachments": [] }
```

- `drafts create`:

```json
{ "draft": { "emailId": "M...", "threadId": "T..." }, "uploaded": [ { "blobId": "B...", "type": "...", "size": 123 } ] }
```

- `drafts send`:

```json
{ "draft": { "emailId": "M...", "threadId": "T..." }, "submission": { "id": "S...", "sendAt": null } }
```

- `drafts update` (metadata-only):

```json
{ "draft": { "emailId": "M...", "threadId": "T..." } }
```

- `drafts rewrite` (content; id may change):

```json
{ "draft": { "emailId": "M...", "threadId": "T..." }, "uploaded": [ { "blobId": "B...", "type": "...", "size": 123 } ], "replacedFrom": "M..." }
```

- `drafts delete` (remove from Drafts mailbox; non-destructive):

```json
{ "deleted": ["M..."] }
```

- `drafts destroy` (permanent; requires `--force`):

```json
{ "destroyed": ["M..."] }
```

---

## 8) History / watch outputs

### 8.1 `xin history` (bootstrap)

When called with no `--since` and no `--page`, xin returns the current Email collection state and an empty change set.

```json
{
  "sinceState": "S...",
  "newState": "S...",
  "hasMoreChanges": false,
  "changes": {
    "created": [],
    "updated": [],
    "destroyed": []
  }
}
```

### 8.2 `xin history --since <state> [--max N] [--page TOKEN]`

Uses JMAP `Email/changes` to return IDs changed since `sinceState`.

```json
{
  "sinceState": "S0",
  "newState": "S1",
  "hasMoreChanges": true,
  "changes": {
    "created": ["M..."] ,
    "updated": ["M..."],
    "destroyed": ["M..."]
  },
  "hydrated": {
    "created": [ {"emailId":"M...","threadId":"T..."} ],
    "updated": [ {"emailId":"M...","threadId":"T..."} ]
  }
}
```

Paging:
- If `hasMoreChanges=true`, xin sets `meta.nextPage` to an opaque token.
- Continue with: `xin history --page <TOKEN>` (and the same `--max`).

Notes:
- Default: `history` returns **IDs only** (it does not explain what fields changed). Use `xin get` to hydrate.
- With `--hydrate`, `history` additionally includes a `hydrated` object containing per-email summary items for `created` and `updated` ids.

### 8.3 `xin watch ...` (stream)

`watch` outputs a **stream** of JSON events. By default this is NDJSON (one JSON object per line).

Event types:

- `ready`:

```json
{ "type": "ready", "sinceState": "S...", "maxChanges": 100 }
```

- `tick` (emitted only when at least one change exists):

```json
{
  "type": "tick",
  "sinceState": "S0",
  "newState": "S1",
  "hasMoreChanges": false,
  "counts": { "created": 1, "updated": 0, "destroyed": 0 }
}
```

- `email.change`:

```json
{ "type": "email.change", "changeType": "created", "id": "M...", "newState": "S1" }
```

`changeType` is one of: `created|updated|destroyed`.

- `email.hydrated` (only when `--hydrate` is set, and only when there are created/updated ids):

```json
{
  "type": "email.hydrated",
  "newState": "S1",
  "hydrated": {
    "created": [ {"emailId":"M...","threadId":"T..."} ],
    "updated": [ {"emailId":"M...","threadId":"T..."} ]
  }
}
```

- `stopped` (Ctrl-C):

```json
{ "type": "stopped", "reason": "ctrl_c" }
```

Notes:
- `--pretty` prints pretty JSON (multi-line) for humans; it is not NDJSON.
- By default, after the stream ends, xin prints the standard envelope as the final line.
- With `--no-envelope`, xin does **not** print the final envelope; on failures it emits a single `{"type":"error","error":{...}}` event and exits non-zero.

---

## 9) Notes / TBD

- Exact `Email/get` properties requested from servers will vary; xin should keep the output normalized and stable.
- Some servers may omit counts or certain fields; in that case, xin should set them to null rather than changing shape.
