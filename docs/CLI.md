# xin CLI Spec (JMAP) — gog gmail parity

This is the **command/flag contract** for `xin`.

- Target audience: humans + agents (JSON-first).
- Design goal: **match the ergonomics of `gog gmail`**, but operate on **JMAP** accounts.

> Notes:
> - Anything marked **(TBD)** is intentionally left blank for now (needs research or provider support).
> - Anything marked **(PLUS)** is a deliberate improvement over `gog gmail`.

---

## 0) Global flags (mirror gog)

- `--help`, `-h`
- Output:
  - `--json` : JSON to stdout (default recommended)
  - `--plain`: stable, parseable text (TSV-like). **Optional**; can be added later.
- Safety:
  - `--force`: skip confirmations for destructive commands
  - `--no-input`: never prompt; fail instead
  - `--dry-run`: show intended changes without applying (**PLUS**, recommended for modify/archive/trash)
- Account selection:
  - `--account <name>`: choose a configured JMAP account (required when multiple)
- Logging:
  - `--verbose`
- `--version`

### Config

TBD (proposal): `~/.config/xin/config.json` (or macOS `~/Library/Application Support/xin/config.json`).

Config should support multiple accounts:

```jsonc
{
  "accounts": {
    "fastmail": {
      "sessionUrl": "https://api.fastmail.com/.well-known/jmap",
      "auth": { "type": "bearer", "tokenEnv": "XIN_FASTMAIL_TOKEN" }
    }
  },
  "defaults": { "account": "fastmail" }
}
```

---

## 1) Read

### 1.1 `xin search [<query>] [--max N] [--page TOKEN] [--oldest] [--filter-json '<json>']`
**gog analog:** `gog gmail search` (threads)

- Returns: list of **thread-like** results by default (see Threading below).

Threading (fixed for v0):
- xin uses JMAP `Email/query` with `collapseThreads=true` by default.
- Each result item corresponds to **at most one Email per Thread** in the matching set.
- Provide:
  - `--collapse-threads=true|false` (default: true)
  - `xin messages search ...` for explicit per-email results

Sorting (fixed for v0):
- Default sort: `receivedAt desc` (newest first).
- Provide `--sort receivedAt` (v0 only). (TBD: additional sort keys.)

Paging token (fixed for v0):
- JMAP paging is based on `position/anchor/limit` semantics.
- xin defines `--page TOKEN` as an opaque cursor (base64url-encoded JSON) containing the minimal state needed to continue.
- Cursor contents (proposal): `{ "anchor": <id|null>, "position": <int>, "limit": <int>, "collapseThreads": <bool>, "sort": <...>, "filterHash": <...> }`.

Mailbox resolution (fixed for v0):
- `in:<mailbox>` resolves in this order:
  1) mailbox **role** match (e.g. role=`inbox`, `trash`, `spam`)
  2) exact name match
  3) case-insensitive name match
  4) explicit id (if value already looks like a JMAP Id)
- This avoids hard-coding localized names.

Query support policy (updated choice):

1) **Prefer native JMAP filter JSON** (first-class, recommended for agents)
- `--filter-json '<JMAP Email/query filter JSON>'`
- This is the most precise and portable way to search across JMAP servers.

2) **Provide a small xin query sugar** (optional, human-friendly)
- `<query>` is a xin-defined DSL that is intentionally **easy to map** to JMAP.
- It is **not** meant to be compatible with Gmail query language.
- Implementation: parse sugar → compile to the same JMAP filter JSON shape.

#### Sugar DSL v0 (must be 1:1 mappable)

Operators:

- Addressing:
  - `from:<text>` / `to:<text>` / `cc:<text>` / `bcc:<text>`
- Content:
  - `subject:<text>`
  - `text:<text>` (explicit full-text)
- Time:
  - `after:<YYYY-MM-DD>`
  - `before:<YYYY-MM-DD>`
- State/keywords:
  - `seen:true|false` → `$seen`
  - `flagged:true|false` → `$flagged`
- Mailbox:
  - `in:<mailboxNameOrId>`
- Attachments:
  - `has:attachment` (alias of `hasAttachment:true`)
  - `hasAttachment:true|false`

Boolean logic (fixed):

JMAP natively supports boolean filter composition via the core **FilterOperator** type:

```json
{ "operator": "AND"|"OR"|"NOT", "conditions": [ <filter>, <filter>, ... ] }
```

(Defined in RFC 8620 §5.5 `/query`; Email filter fields are defined in RFC 8621 §4.4.1.)

xin sugar maps directly to this model:

- AND: implicit by whitespace
  - Example: `from:github seen:false` → AND of two conditions (may be merged into a single FilterCondition object).
- Grouping (AND): parentheses `(...)`
  - Example: `(from:github subject:Kingfisher) in:INBOX`
- OR: `or:( <expr> | <expr> | ... )`
  - Example: `or:(from:github | from:atlassian) seen:false`
- NOT: prefix `-` for a term or group
  - Example: `-in:Trash`, `-(from:spam subject:"sale")`

Parsing/precedence rules (v0 implementation constraints):
- OR exists **only** inside `or:(...)` (no bare `OR` keyword), to keep parsing deterministic.
- `or:(...)` is **not nestable** in v0. Inside it, only simple terms are allowed.
- NOT (`-`) applies to the **next simple term** in v0. Group negation `-(...)` is reserved for a future version.
- Parentheses grouping is reserved for a future version. (We keep the mapping documented, but do not implement it in v0.)
- Quoted values `"..."` are supported for operator values (e.g. `subject:"foo bar"`).

#### Compilation (examples)

Simple AND (often merged into one FilterCondition):

- `from:github subject:"Kingfisher" seen:false` →

```json
{ "from": "github", "subject": "Kingfisher", "notKeyword": "$seen" }
```

Mailbox + time:

- `in:INBOX has:attachment after:2026-01-01` →

```json
{ "inMailbox": "<resolved-mailbox-id>", "hasAttachment": true, "after": "2026-01-01T00:00:00Z" }
```

OR composition (uses FilterOperator):

- `or:(from:github | from:atlassian) seen:false` →

```json
{
  "operator": "AND",
  "conditions": [
    {
      "operator": "OR",
      "conditions": [
        { "from": "github" },
        { "from": "atlassian" }
      ]
    },
    { "notKeyword": "$seen" }
  ]
}
```

NOT composition (uses FilterOperator):

- `-(from:spam subject:"sale")` →

```json
{ "operator": "NOT", "conditions": [ { "from": "spam", "subject": "sale" } ] }
```

Notes:
- `in:<...>` requires mailbox name→id resolution via `Mailbox/get`.
- Time values must be RFC3339; sugar uses a date and xin expands it (timezone handling TBD; default local).
- If both `<query>` and `--filter-json` are provided, `--filter-json` wins.
- For AND-only expressions, xin MAY emit a single FilterCondition object with multiple properties (RFC 8621 states multiple properties are equivalent to AND).

JSON output fields (proposal):
- `items[]`: `{ threadId, latestEmailId, subject, from, to, date, snippet, unread, hasAttachment, mailboxIds, keywords }`
- `nextPageToken`

**TBD:** precise pagination semantics depend on JMAP `Email/query` (`position`, `anchor`, `limit`).

### 1.2 `xin messages search <query> [--max N] [--page TOKEN]`
**gog analog:** `gog gmail messages search` (messages)

- Returns individual `Email` items (not grouped).

### 1.3 `xin get <emailId> [--format full|metadata|raw] [--headers a,b,c]`
**gog analog:** `gog gmail get <messageId>`

- `--format=metadata`: fetch headers only (fast)
- `--format=full`: includes body (as available/normalized)
- `--format=raw`: return provider raw JMAP `Email` object

**TBD:** body normalization rules across providers (text/plain vs html, inlined parts).

### 1.4 `xin thread get <threadId> [--full]`
**gog analog:** `gog gmail thread get <threadId>`

- Returns all emails in a thread, optionally with bodies.

**TBD:** if provider does not support stable threads, xin may provide a best-effort grouping.

### 1.5 `xin thread attachments <threadId>`
**gog analog:** `gog gmail thread attachments <threadId>`

- Lists attachments across emails in the thread.
- Output should include: `{ emailId, blobId, name, type, size }`

### 1.6 `xin attachment <emailId> <blobId> [--out PATH] [--name FILENAME]`
**gog analog:** `gog gmail attachment <messageId> <attachmentId>`

- Downloads an attachment blob.

### 1.7 `xin url <id>...`
**gog analog:** `gog gmail url <threadId>...`

- Prints webmail URLs (if provider supports).

**TBD:** likely provider-specific and may be unavailable on many JMAP servers.

---

## 2) Organize

### 2.1 `xin thread modify <threadId> --add X --remove Y`
**gog analog:** `gog gmail thread modify <threadId> --add/--remove`

`X/Y` support:
- Mailbox membership (move/copy): `INBOX`, `ARCHIVE`, `TRASH`, `SPAM` (where available)
- Keywords (tags): `$seen`, `$flagged`, plus provider/user keywords

Because Gmail uses labels and JMAP uses mailboxIds + keywords, xin should accept both:
- `--add-mailbox <nameOrId>` / `--remove-mailbox <nameOrId>` (recommended)
- `--add-keyword <kw>` / `--remove-keyword <kw>`

For parity with gog, `--add/--remove` can remain as a convenience that auto-routes:
- If token matches a known mailbox name => mailbox op
- Else => keyword op

### 2.2 `xin batch modify <emailId>... --add X --remove Y`
**gog analog:** `gog gmail batch modify <messageId>...`

- Same semantics as thread modify, but for individual emails.

### 2.3 `xin batch delete <emailId>...`
**gog analog:** `gog gmail batch delete` (permanent delete)

- **Not recommended**. Prefer `xin trash`.
- Should require `--force`.

**TBD:** some providers may not allow true delete beyond moving to Trash.

### 2.4 Convenience commands (PLUS)

These are intentionally **more ergonomic** than `gog` while still mappable:

- `xin archive <id>...`  
  Sugar for “remove from INBOX / move to Archive mailbox”.

- `xin read <id>...`  
  Sugar for adding `$seen`.

- `xin unread <id>...`

- `xin trash <id>...`

Notes:
- Accept both `emailId` and `threadId` where possible; xin can disambiguate via prefix:
  - `email:<id>` / `thread:<id>` (proposal)

---

## 3) Labels (Mailboxes)

### 3.1 `xin labels list`
**gog analog:** `gog gmail labels list`

- For JMAP, this lists **Mailboxes**.

### 3.2 `xin labels get <mailboxIdOrName>`
**gog analog:** `gog gmail labels get ...`

- Return mailbox details including unread/total counts when available.

### 3.3 `xin labels create <name>`
**gog analog:** `gog gmail labels create <name>`

### 3.4 `xin labels modify <mailboxId> ...` (TBD)
**gog analog:** `gog gmail labels modify ...`

Gmail “modify labels on threads” doesn’t map cleanly here; in JMAP labels==mailboxes.

Proposal:
- Keep `labels modify` for mailbox properties (rename, parent, sortOrder) rather than message tagging.

---

## 4) Write

### 4.1 `xin send --to ... --subject ... --body-file ... [--cc ...] [--bcc ...] [--attach ...]`
**gog analog:** `gog gmail send ...`

Reply support (parity):
- `--reply-to-email-id <id>`
- `--thread-id <id>`
- `--reply-all`

Implementation model (JMAP):
- Create draft via `Email/set` into Drafts mailbox
- Send via `EmailSubmission/set`

**TBD:** “from alias” / identities handling differs by provider.

### 4.2 `xin drafts list|get|create|update|delete|send`
**gog analog:** `gog gmail drafts ...`

Drafts are just emails in the Drafts mailbox.

---

## 5) History / watch

### 5.1 `xin history [--since <state>] [--max N] [--page TOKEN]`
**gog analog:** `gog gmail history`

- Uses JMAP `Email/changes` to return changed/created/destroyed since state.

### 5.2 `xin watch ...` (TBD, PLUS)

- If provider supports JMAP WebSocket push (RFC 8887), xin can expose a `watch` surface.
- Otherwise, xin can provide a polling helper (agent can schedule).

---

## 6) gog features that xin will *not* mirror (by default)

- `gog gmail settings ...` (Gmail-only admin)
- Gmail-specific smart fields (category, importance signals)

---

## 7) Where xin can exceed gog (summary)

- First-class, stable JSON schema for agents.
- More consistent batch operations via JMAP methodCalls/backreferences.
- Portable incremental sync via `*/changes` (and optional WebSocket push).
- Convenience commands (`archive/read/unread/trash`) without forcing callers to remember label/keyword mechanics.
