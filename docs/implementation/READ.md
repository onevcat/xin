# Implementation: READ

This document covers implementing read-only commands:

- `xin search`
- `xin messages search`
- `xin get`
- `xin thread get`
- `xin thread attachments`
- `xin attachment` (download)

References:
- RFC 8620 (JMAP Core): Request format, /query, method result references
- RFC 8621 (JMAP Mail): `Email/query`, `Email/get`, `Thread/get`, `SearchSnippet/get`

---

## 0) Fetch Session (RFC 8620 §2)

HTTP:
- `GET SESSION_URL` (authenticated)

From the Session response we need:
- `apiUrl` (POST JMAP requests here)
- `uploadUrl` and `downloadUrl` templates
- `primaryAccounts["urn:ietf:params:jmap:mail"]` → `accountId`

xin should cache Session per account for a short time (TBD).

---

## 1) `xin labels list` / mailbox resolution helper

Many read commands need mailbox ids.

Call `Mailbox/get` with `ids: null` to fetch all mailboxes, then build:
- `role -> mailboxId`
- `name -> mailboxId`

JMAP request:

```json
{
  "using": [
    "urn:ietf:params:jmap:core",
    "urn:ietf:params:jmap:mail"
  ],
  "methodCalls": [
    ["Mailbox/get", {"accountId": "A", "ids": null}, "c1"]
  ]
}
```

---

## 2) `xin search` (thread-like results)

CLI contract says v0 uses:
- `Email/query` with `collapseThreads=true` by default
- sort default: `receivedAt desc`

### 2.1 Compile filter

- If user provides `--filter-json`, parse it.
- Otherwise parse sugar DSL and compile to `FilterCondition` / `FilterOperator`.

### 2.2 Email/query

Request:

```json
{
  "using": ["urn:ietf:params:jmap:core","urn:ietf:params:jmap:mail"],
  "methodCalls": [
    ["Email/query", {
      "accountId": "A",
      "filter": {"inMailbox": "<inboxMailboxId>"},
      "sort": [{"property": "receivedAt", "isAscending": false}],
      "collapseThreads": true,
      "position": 0,
      "limit": 20
    }, "q1"]
  ]
}
```

Response gives:
- `ids`: emailIds (representative per thread when collapseThreads=true)
- `position`, `total` (optional)

### 2.3 Email/get to hydrate summaries

Immediately follow with `Email/get` for the returned ids.

Recommended minimal properties for `SCHEMA.md §4.1`:
- `id`, `threadId`, `receivedAt`, `subject`, `from`, `to`, `preview`, `hasAttachment`, `mailboxIds`, `keywords`

Request (single JMAP request can include both methodCalls):

```json
{
  "using": ["urn:ietf:params:jmap:core","urn:ietf:params:jmap:mail"],
  "methodCalls": [
    ["Email/query", {"accountId":"A", "filter":{}, "sort":[{"property":"receivedAt","isAscending":false}], "collapseThreads":true, "position":0, "limit":20}, "q1"],
    ["Email/get", {
      "accountId": "A",
      "ids": "#q1/ids",
      "properties": [
        "id","threadId","receivedAt","subject",
        "from","to","preview","hasAttachment",
        "mailboxIds","keywords"
      ]
    }, "g1"]
  ]
}
```

Map each Email to `search.items[]`:
- `emailId` = `id`
- `unread` = absence of `$seen` in `keywords`

### 2.4 Optional: SearchSnippet/get

If we want better snippet highlighting (TBD), use `SearchSnippet/get` with the same filter.
This is optional and server may return `unsupportedFilter`.

---

## 3) `xin messages search`

Same as `xin search` but with:
- `collapseThreads=false`

Hydration still uses `Email/get`.
Output reuses the same item shape (SCHEMA.md §4.2).

---

## 4) `xin get <emailId>`

Use `Email/get`.

- `--format metadata`: request summary properties only
- `--format full`: also request body structure and body values
  - Use JMAP `maxBodyValueBytes` to keep output bounded.
  - **xin default:** 262144 (256KiB) per body value.
  - If the server truncates, it will return `EmailBodyValue.isTruncated=true`.
    - xin surfaces this as:
      - `data.body.textMeta.isTruncated` / `data.body.htmlMeta.isTruncated`
      - a human-readable entry in `meta.warnings[]`
  - Callers can re-run with `--max-body-bytes <N>`.
- `--format raw`: include raw pass-through in output (`data.raw`)
- `--headers a,b,c`: request additional RFC 5322 header fields and return a parsed dictionary in `data.email.headers` (SCHEMA v0).
  - Uses JMAP computed properties: `header:<Name>:asText`.
  - For repeatable headers (e.g. `received`, `dkim-signature`, `authentication-results`, `resent-*`), xin requests `header:<Name>:asText:all` and returns an array.
  - For common structured fields, xin prefers the standard JMAP properties when available:
    - `message-id` / `in-reply-to` / `references`
    - `from` / `to` / `cc` / `bcc` / `reply-to` / `sender`
    - `date`

Typical full request:

```json
{
  "using": ["urn:ietf:params:jmap:core","urn:ietf:params:jmap:mail"],
  "methodCalls": [
    ["Email/get", {
      "accountId": "A",
      "ids": ["M123"],
      "properties": [
        "id","threadId","receivedAt","subject",
        "from","to","cc","bcc",
        "mailboxIds","keywords","hasAttachment","preview",
        "bodyStructure","bodyValues","textBody","htmlBody","attachments"
      ],
      "bodyProperties": ["partId","blobId","size","name","type","disposition","cid"],
      "fetchTextBodyValues": true,
      "fetchHTMLBodyValues": true
    }, "g1"]
  ]
}
```

Then normalize to `SCHEMA.md §4.3`.

### Body selection strategy (v0, implemented)

When `--format full` is used, xin requests `textBody`, `htmlBody`, and `bodyValues` (with `fetchTextBodyValues=true`, `fetchHTMLBodyValues=true`).

Normalization rules (v0):

- `data.body.text`:
  - If `Email.textBody` exists and is non-empty, take **the first** part’s `partId`, and look up `Email.bodyValues[partId].value`.
  - Otherwise set to `null`.
- `data.body.html`:
  - If `Email.htmlBody` exists and is non-empty, take **the first** part’s `partId`, and look up `Email.bodyValues[partId].value`.
  - Otherwise set to `null`.
- We do **not** concatenate multiple body parts in v0.
  - (Future: we may join multiple parts with separators, but only if we can keep output stable and predictable.)

Truncation surfacing (v0):

- If `EmailBodyValue.isTruncated=true`, xin sets:
  - `data.body.textMeta.isTruncated=true` (or `htmlMeta`)
  - and adds a warning in `meta.warnings[]`.

Attachments (v0):

- `--format full` returns **attachment metadata only** in `data.attachments[]` (blobId/name/type/size/disposition).
- It never embeds attachment bytes in JSON.
- Callers must download via `xin attachment <emailId> <blobId> --out ...`.

---

## 5) `xin thread get <threadId>`

Use `Thread/get` to get `emailIds`, then `Email/get`.

```json
{
  "using": ["urn:ietf:params:jmap:core","urn:ietf:params:jmap:mail"],
  "methodCalls": [
    ["Thread/get", {"accountId":"A", "ids": ["T123"]}, "t1"],
    ["Email/get", {"accountId":"A", "ids": "#t1/list/0/emailIds", "properties": ["id","threadId","receivedAt","subject","from","to","preview","hasAttachment","mailboxIds","keywords"]}, "g1"]
  ]
}
```

Map to `SCHEMA.md §4.4`.

---

## 6) Attachments list + download

### 6.1 List attachments for a thread

Either:
- Use `xin thread get` with `attachments` properties, or
- Call `Email/get` with `attachments` and `bodyProperties`.

### 6.2 Download attachment

Use Session `downloadUrl` template (RFC 8620 §6.2) with:
- `accountId`, `blobId`, `type`, `name`

xin should produce the download URL and then GET it with auth and write to disk.
