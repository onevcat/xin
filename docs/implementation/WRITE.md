# Implementation: WRITE

Covers:

- `xin identities list/get`
- `xin send`
- `xin drafts create/send`
- attachment upload + reference

References:
- RFC 8620 §6.1 uploadUrl (binary upload)
- RFC 8621 §4.6 Email/set (create draft)
- RFC 8621 §7 EmailSubmission + EmailSubmission/set

---

## 0) Preconditions

- Fetch Session → `apiUrl`, `uploadUrl`, `accountId`
- Ensure `using` includes:
  - `urn:ietf:params:jmap:core`
  - `urn:ietf:params:jmap:mail`
  - `urn:ietf:params:jmap:submission`

Per RFC-first principle, xin sends the request; if the server rejects due to missing capability, surface the error.

---

## 1) `xin identities list`

Use `Identity/get`.

```json
{
  "using": ["urn:ietf:params:jmap:core","urn:ietf:params:jmap:submission"],
  "methodCalls": [
    ["Identity/get", {"accountId":"A", "ids": null}, "i1"]
  ]
}
```

Normalize to `SCHEMA.md §7.1`.

---

## 2) Upload attachments (RFC 8620 §6.1)

For each `--attach <path>`:

- POST to `uploadUrl{accountId}`
- Set `Content-Type` header
- Body: file bytes

Response:

```json
{ "accountId": "A", "blobId": "B123", "type": "application/pdf", "size": 12345 }
```

Collect uploaded blobs into the output (`SCHEMA.md §7.2.uploaded`).

---

## 3) Create draft via Email/set (RFC 8621 §4.6)

### 3.1 Resolve Drafts mailbox id (v0)

- Prefer mailbox with `role="drafts"`.
- Fallback to name match (case-insensitive) when roles are absent.

The resolved Drafts mailbox id MUST be included in `mailboxIds` on create.

### 3.2 Build deterministic MIME structure (v0)

xin uses a deterministic `bodyStructure` + `bodyValues` layout for both `send` and draft creation.

Rules:

- If plain+html: create `multipart/alternative` as the body.
- If any attachments exist: wrap in top-level `multipart/mixed` and append attachment parts.
- Attachment parts MUST include `disposition="attachment"` and SHOULD include `name`.

In JMAP terms:

- Use `bodyStructure` (tree of `EmailBodyPart`).
- Use `bodyValues` keyed by `partId` for text/html bodies.
- Use `blobId` for attachment parts (from upload).

Implementation note (portability):

- Avoid setting `textBody/htmlBody` alongside `bodyStructure`.
- Prefer `bodyStructure` + `bodyValues` (+ `blobId` for attachments), which is portable across JMAP servers and avoids server-specific rejection modes for “mixed representations”.

### 3.3 Extra headers (reply/forward) (v0)

Some write flows need additional RFC822 headers (threading, etc.).

xin supports this by adding raw headers during `Email/set(create)`:

- `In-Reply-To: <original-message-id>`
- `References: <existing-refs> <original-message-id>`

These headers are supplied by higher-level commands (e.g. `xin reply`) and injected via `Email/set` create using `Header::as_raw`.

### 3.4 Email/set create (simplified)

```json
{
  "using": ["urn:ietf:params:jmap:core","urn:ietf:params:jmap:mail"],
  "methodCalls": [
    ["Email/set", {
      "accountId": "A",
      "create": {
        "d1": {
          "mailboxIds": {"<draftsMailboxId>": true},
          "keywords": {"$draft": true},
          "subject": "Hello",
          "to": [{"name": null, "email": "you@example.com"}],
          "bodyStructure": { /* deterministic MIME layout */ },
          "bodyValues": { /* for partId-backed bodies */ }
        }
      }
    }, "e1"]
  ]
}
```

The response `created.d1.id` is the new draft `emailId`.

---

## 4) Send via EmailSubmission/set (RFC 8621 §7.5)

Create an EmailSubmission referencing the emailId and an identityId.

- If user passed `--identity`, pick that Identity (match by Identity id or email).
- Otherwise pick the first identity returned by `Identity/get`.

The RFC allows `envelope` to be null/omitted; server must derive recipients from headers.

Example (combined request using backreference):

```json
{
  "using": [
    "urn:ietf:params:jmap:core",
    "urn:ietf:params:jmap:mail",
    "urn:ietf:params:jmap:submission"
  ],
  "methodCalls": [
    ["Email/set", {"accountId":"A", "create": {"d1": { /* draft */ }}}, "e1"],
    ["EmailSubmission/set", {
      "accountId": "A",
      "create": {
        "s1": {
          "identityId": "I123",
          "emailId": "#e1/created/d1/id"
        }
      }
    }, "s2"]
  ]
}
```

Normalize to `SCHEMA.md §7.2`.

---

## 5) Drafts list

`xin drafts list` is implemented as a read-only query against the Drafts mailbox.

Implementation outline:

1) Resolve Drafts mailbox id (prefer role=`drafts`, then name fallback).
2) `Email/query` with `filter={"inMailbox":"<draftsMailboxId>"}` and `collapseThreads=false`.
3) `Email/get` to hydrate summary fields (same properties as READ search).
4) Paging uses the same `--page` token contract as READ search (position/limit + stable filter).

---

## 6) Drafts get

`xin drafts get <draftEmailId>` is equivalent to `xin get`, but returns the email object under `data.draft`.

- Uses `Email/get`.
- Supports `--format metadata|full|raw`.

---

## 7) Drafts update (metadata-only)

`xin drafts update <draftEmailId> ...` performs an **in-place** `Email/set` update that MUST NOT change the draft id.

Key semantics (v0):

- This command is **metadata-only**: it can modify mailbox membership and keywords.
- It MUST NOT attempt to modify message content (subject/from/to/body/attachments), because JMAP (RFC 8621) defines those Email properties as immutable and servers may reject such updates.

Flags mirror `batch modify` / ORGANIZE semantics:

- `--add/--remove`: auto route (mailbox if resolvable, otherwise keyword)
- `--add-mailbox/--remove-mailbox`
- `--add-keyword/--remove-keyword`

---

## 8) Drafts rewrite (content)

`xin drafts rewrite <draftEmailId> ...` rewrites message content by **creating a new draft** and replacing the old one.

Key semantics (v0):

- Returns a **new** `draft.emailId` (id may change).
- Includes `replacedFrom` to indicate which draft was replaced.
- Default cleanup of the old draft is **non-destructive**:
  - remove Drafts mailbox membership
  - unset `$draft` keyword
- Optional: `--destroy-old` will permanently destroy the replaced draft, but requires global `--force`.

Attachment flags:

- default: keep existing attachments, and append any new uploads.
- `--replace-attachments`: discard existing attachments and use only the newly uploaded ones.
- `--clear-attachments`: remove all attachments.

Body structure rules reuse the same deterministic MIME builder as `send`/`drafts create`.

---

## 9) Drafts send

`xin drafts send <draftEmailId>` only needs EmailSubmission/set:

```json
{
  "using": ["urn:ietf:params:jmap:core","urn:ietf:params:jmap:submission"],
  "methodCalls": [
    ["EmailSubmission/set", {
      "accountId": "A",
      "create": {
        "s1": {"identityId": "I123", "emailId": "M_draft"}
      }
    }, "s1"]
  ]
}
```

---

## 10) Drafts delete (non-destructive)

`xin drafts delete <draftEmailId>...` removes the Email(s) from the Drafts mailbox without destroying the Email object.

Implementation:

1) Resolve Drafts mailbox id (prefer role=`drafts`, then name fallback).
2) Resolve Trash mailbox id (prefer role=`trash`, then name fallback).
3) `Email/set` update patch for each draft:
   - `mailboxIds/<draftsMailboxId>` → false
   - `mailboxIds/<trashMailboxId>` → true
   - `keywords/$draft` → false

Rationale: `Email.mailboxIds` must not become empty; moving to Trash preserves recoverability while removing from Drafts.

Example:

```json
{
  "using": ["urn:ietf:params:jmap:core","urn:ietf:params:jmap:mail"],
  "methodCalls": [
    ["Email/set", {
      "accountId": "A",
      "update": {
        "M1": { "mailboxIds/<draftsMailboxId>": false },
        "M2": { "mailboxIds/<draftsMailboxId>": false }
      }
    }, "e1"]
  ]
}
```

---

## 11) Drafts destroy (permanent)

`xin drafts destroy <draftEmailId>...` permanently deletes the Email object(s) via `Email/set` destroy.

- Destructive: requires `--force`.

Example:

```json
{
  "using": ["urn:ietf:params:jmap:core","urn:ietf:params:jmap:mail"],
  "methodCalls": [
    ["Email/set", {
      "accountId": "A",
      "destroy": ["M1", "M2"]
    }, "e1"]
  ]
}
```
