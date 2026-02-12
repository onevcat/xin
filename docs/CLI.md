# xin CLI Spec (JMAP) — gog gmail parity

This is the **command/flag contract** for `xin`.

- Target audience: humans + agents (JSON-first).
- Design goal: **match the ergonomics of `gog gmail`**, but operate on **standard JMAP** accounts.

JSON contract:
- For `--json` output shapes, see: `docs/SCHEMA.md`.
- Each command below references the relevant schema section.

## Core principle (RFC-first)

xin is a **standards client**:

- xin constructs requests strictly according to the relevant RFCs (JMAP core + JMAP mail/submission).
- xin does **not** implement provider-specific behavior toggles.
- If a provider does not support a method/capability, xin still sends the standard request and then reports the resulting JMAP error in a structured way.

(We will, of course, fetch the JMAP Session to discover `apiUrl`, `accountId`, etc., as required by RFC 8620.)

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
- Input convenience (PLUS):
  - flags that accept JSON (e.g. `--filter-json`) also accept `@/path/to/file.json` to read JSON from file.
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

### Environment

Required connection info:

- `XIN_BASE_URL` (preferred) or `XIN_SESSION_URL`
- Authentication (choose one)
- `XIN_TOKEN` or `XIN_TOKEN_FILE` (Bearer)
- `XIN_BASIC_USER` and `XIN_BASIC_PASS` (or `XIN_BASIC_PASS_FILE`) (Basic)

Optional:

- `XIN_TRUST_REDIRECT_HOSTS` (comma-separated allowlist for session URL redirects)

---

## 1) Read

### 1.1 `xin search [<query>] [--max N] [--page TOKEN] [--oldest] [--filter-json '<json>']`
**gog analog:** `gog gmail search` (threads)
**JSON schema:** SCHEMA.md §4.1

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
- Cursor contents (v0 implemented):
  - `position` (int), `limit` (int)
  - `collapseThreads` (bool)
  - `isAscending` (bool) (from `--oldest`)
  - `filter` (the compiled JMAP filter JSON)

Planned improvements:
- Include explicit sort comparators in the token (v0 currently fixed to `receivedAt`).
- Consider emitting a stable filter hash in the token to keep tokens smaller.

Rule:
- If the user changes filter/sort/collapseThreads while supplying `--page`, xin MUST error instead of producing surprising results.

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
- xin treats this as **server-owned input**: it only parses the JSON, then sends it as-is (no whitelist / no semantic validation). Any server error is surfaced back to the caller.
- This is the most precise and portable way to search across JMAP servers.

2) **Provide a small xin query sugar** (v0 implemented, human-friendly)
- `<query>` is a xin-defined DSL that is intentionally **easy to map** to JMAP.
- It is **not** meant to be compatible with Gmail query language.
- Implementation: parse sugar → compile to the same JMAP filter JSON shape.
- Note: because the CLI positional `<query>` is a single argument, multi-term queries should be quoted:
  - `xin search "from:alice seen:false"`

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
- OR: `or:( <expr> | <expr> | ... )`
  - Example: `or:(from:github | from:atlassian) seen:false`
- NOT: prefix `-` for a **single term**
  - Example: `-in:Trash`, `-seen:true`

Unsupported in v0 (will return a `xinUsageError`):
- Parentheses grouping `(...)`
- Group negation `-(...)`
- Nested `or:(...)`

Parsing/precedence rules (v0 implementation constraints):
- OR exists **only** inside `or:(...)` (no bare `OR` keyword), to keep parsing deterministic.
- `or:(...)` is **not nestable** in v0. Inside it, only simple terms are allowed.
- NOT (`-`) applies to the **next simple term** in v0.
- Quoted values `"..."` are supported for operator values (e.g. `subject:"foo bar"`).

If you need full boolean expressions, use `--filter-json`.

Examples:
- Inline JSON:
  - `--filter-json '{"operator":"NOT","conditions":[{"inMailbox":"<mbx>"}]}'`
- File:
  - `--filter-json @filter.json`

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

NOT composition:

- v0 supports NOT only for a single term via `-term` (e.g. `-in:Trash`).
- Group negation `-(...)` is **not implemented** in v0.

Notes:
- `in:<...>` requires mailbox name→id resolution via `Mailbox/get`.
- Time values:
  - v0 accepts `YYYY-MM-DD` (expanded to `00:00:00Z`) or full RFC3339.
- If both `<query>` and `--filter-json` are provided, `--filter-json` wins.
- For AND-only expressions, xin MAY emit a single FilterCondition object with multiple properties (RFC 8621 states multiple properties are equivalent to AND).

JSON output fields (proposal):
- `items[]`: `{ threadId, latestEmailId, subject, from, to, date, snippet, unread, hasAttachment, mailboxIds, keywords }`
- `nextPage` (in `meta.nextPage`): string|null (opaque cursor for `--page`)

Paging note (v0):
- Some servers omit `total` in `Email/query`. xin still emits `nextPage` when the result count hits `--max`.
- If results are fewer than `--max`, xin assumes there is no next page.

Next improvements (planned):
- Add more sort keys.
- Make the page token include explicit sort comparators (instead of the current fixed v0 behavior).
- Consider emitting a stable filter hash in the token to keep tokens smaller.

**TBD:** precise pagination semantics depend on JMAP `Email/query` (`position`, `anchor`, `limit`).

### 1.2 `xin messages search <query> [--max N] [--page TOKEN]`
**gog analog:** `gog gmail messages search` (messages)
**JSON schema:** SCHEMA.md §4.2

- Returns individual `Email` items (not grouped).

### 1.3 `xin get <emailId> [--format full|metadata|raw] [--max-body-bytes N] [--headers a,b,c]`
**gog analog:** `gog gmail get <messageId>`
**JSON schema:** SCHEMA.md §4.3

- `--format=metadata`: fetch headers only (fast)
- `--format=full`: includes body (as available/normalized)
  - Default `maxBodyValueBytes` = 262144 (256KiB) per body value.
  - If truncated, xin will:
    - set `data.body.textMeta.isTruncated` / `data.body.htmlMeta.isTruncated` to true
    - add a human-readable warning string into `meta.warnings[]`
  - To fetch more, re-run with `--max-body-bytes <N>`.
- `--format=raw`: return provider raw JMAP `Email` object

**TBD:** body normalization rules across providers (text/plain vs html, inlined parts).

### 1.4 `xin thread get <threadId> [--full]`
**gog analog:** `gog gmail thread get <threadId>`
**JSON schema:** SCHEMA.md §4.4

- Returns all emails in a thread.
- With `--full`, `emails[]` items use the same shape as `xin get --format full` (body + attachments metadata; no bytes inlined).

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

### 1.8 `xin inbox next [--all] [--oldest] [--max <n>] [--page <token>] [<query>]` (v0)

Inbox-zero helper: return the next email to process from Inbox.

- Default compiled query: `in:inbox seen:false` (unread only).
- `--all`: include read messages.
- `--oldest`: oldest-first (default newest-first).
- `<query>`: additional sugar terms appended with AND.

Output is `messages search`-compatible, with a convenience `data.item` for the first result.

### 1.9 `xin inbox do <emailId> <archive|trash|read|unread> [--whole-thread]` (v0)

Inbox-zero helper: apply an action to an email (optionally to its whole thread).

- Maps to existing sugar commands:
  - `archive` → `xin archive <emailId>`
  - `trash` → `xin trash <emailId>`
  - `read` → `xin read <emailId>`
  - `unread` → `xin unread <emailId>`
- `--whole-thread`: same semantics/constraints as the sugar commands.

---

## 2) Organize

**JSON schema (organize):** SCHEMA.md §6


### 2.1 `xin thread modify <threadId> --add X --remove Y`
**gog analog:** `gog gmail thread modify <threadId> --add/--remove`

`X/Y` support:
- Mailbox membership (move/copy): via `mailboxIds`
- Keywords (tags): via `keywords` (e.g. `$seen`, `$flagged`)

Flags:
- `--add-mailbox <nameOrId>` / `--remove-mailbox <nameOrId>`
- `--add-keyword <kw>` / `--remove-keyword <kw>`
- Convenience: `--add/--remove` auto-routes (mailbox name/id/role → mailbox op; otherwise keyword op)

Implementation plan (v0, fixed — RFC-compliant):

- JMAP does **not** define an `inThread` filter for `Email/query` in RFC 8621.
- The RFC-defined way to expand a thread into its emails is `Thread/get` (RFC 8621 §3.1), which returns `emailIds`.

To modify a thread, xin does:
1) `Thread/get` for `<threadId>` → obtain `emailIds`
2) apply the modification to those emailIds via `Email/set` (batch update; patch syntax)

Error behavior:
- If the server returns a standard error (e.g. unknown method/capability, forbidden, notFound), xin surfaces it as structured output.

### 2.2 `xin batch modify <emailId>... --add X --remove Y`
**gog analog:** `gog gmail batch modify <messageId>...`

- Same semantics as thread modify, but for individual emails.

### 2.3 `xin batch delete <emailId>...` (v0)
**gog analog:** `gog gmail batch delete` (permanent delete)

- **Destructive**: requires `--force`.
- Uses `Email/set` destroy.
- Some providers may not allow true delete beyond moving to Trash; xin will surface the standard error.

### 2.4 `xin thread delete <threadId>` (v0)

- **Destructive**: requires `--force`.
- Expands the thread via `Thread/get` → obtains `emailIds` → destroys them via `Email/set` destroy.

### 2.4 Convenience commands (PLUS)

These are intentionally ergonomic **but keep the CLI type system explicit**:

- `xin ...` commands operate on **Email ids**
- `xin thread ...` commands operate on **Thread ids**

Key RFC concepts:
- Inbox/Trash/Archive/etc are **Mailboxes** (often identified via `Mailbox.role`).
- Read/starred/etc are **keywords** (`$seen`, `$flagged`, ...).

Mailbox roles referenced here are the lowercase versions of IMAP special-use attributes (RFC 6154 / IANA registry), e.g. `inbox`, `drafts`, `sent`, `trash`, `junk`, `archive`.

#### Email-level sugar (v0)

All email-level sugar commands accept an optional (PLUS) flag:
- `--whole-thread`: apply the action to the entire thread containing the given email.
  - v0 constraint: only allowed when exactly **one** emailId is provided.
  - Implementation: `Email/get` → obtain `threadId` → perform the corresponding `xin thread ...` action (`Thread/get` → `Email/set`).

- `xin archive <emailId>... [--whole-thread]`
  - Convention: remove membership of the mailbox with role `inbox`.
  - If a mailbox with role `archive` exists, xin MAY also add it; otherwise archive is represented purely as “not in inbox”.

- `xin read <emailId>... [--whole-thread]`
  - Add keyword `$seen`.

- `xin unread <emailId>... [--whole-thread]`
  - Remove keyword `$seen`.

- `xin trash <emailId>... [--whole-thread]`
  - RFC 8621 guidance for “delete to trash”: set `mailboxIds` to contain **only** the mailbox with role `trash`.

#### Thread-level sugar (v0)

These mirror the email-level commands, but expand the thread via `Thread/get` then apply the change to each Email via `Email/set`.

- `xin thread archive <threadId>...`
- `xin thread read <threadId>...`
- `xin thread unread <threadId>...`
- `xin thread trash <threadId>...`

Role mapping aliases (v0):
- treat `spam` as an alias for role `junk`.
- treat `bin` as an alias for role `trash`.

Notes:
- Accept both `emailId` and `threadId` where possible; xin can disambiguate via prefix:
  - `email:<id>` / `thread:<id>` (proposal)

---

## 3) Labels (Mailboxes)

Naming / aliasing (fixed):
- In xin, **labels are mailboxes** (RFC 8621 `Mailbox`).
- For ergonomics:
  - `xin labels ...` and `xin mailboxes ...` are **exact aliases** (same commands, same flags, same output).

### 3.1 `xin labels list` / `xin mailboxes list`
**gog analog:** `gog gmail labels list`
**JSON schema:** SCHEMA.md §5.1

- Lists all mailboxes (via `Mailbox/get` with `ids=null`).

### 3.2 `xin labels get <mailboxId|name|role>` / `xin mailboxes get ...`
**gog analog:** `gog gmail labels get ...`
**JSON schema:** SCHEMA.md §5.2

- Returns mailbox details including unread/total counts when available.
- Resolution order (v0):
  1) exact id match
  2) role match (e.g. `inbox`, `drafts`, `sent`, `trash`, `junk`)
  3) exact name match
  4) case-insensitive name match

### 3.3 `xin labels create <name> [--parent <mailboxId>] [--role <role>] [--subscribe true|false]`
**gog analog:** `gog gmail labels create <name>`
**JSON schema:** SCHEMA.md §5.3

- Creates a mailbox via `Mailbox/set`.
- Notes:
  - Setting `--role` may be rejected by servers; xin will surface the error.
  - `--subscribe` maps to `isSubscribed`.

### 3.4 `xin labels rename <mailboxId> --name <newName>` (v0)
**JSON schema:** SCHEMA.md §5.3

- Convenience wrapper around `Mailbox/set` update.

### 3.5 `xin labels delete <mailboxId> [--remove-emails]` (v0)
**JSON schema:** SCHEMA.md §5.3

- Destroys a mailbox via `Mailbox/set` destroy.
- If `--remove-emails` is set, xin sets `onDestroyRemoveEmails=true` (RFC 8621 `Mailbox/set`).
- If `--remove-emails` is not set, xin uses the RFC default (`false`).

### 3.6 `xin labels modify <mailboxId> [--name <str>] [--parent <mailboxId>] [--sort-order <int>] [--subscribe true|false]` (v0)
**JSON schema:** SCHEMA.md §5.3

Gmail’s “modify labels on threads” does not map to JMAP.

In xin, `labels modify` changes **mailbox properties** via `Mailbox/set` update:
- `--name`: maps to `Mailbox.name`
- `--parent`: maps to `Mailbox.parentId`
- `--sort-order`: maps to `Mailbox.sortOrder`
- `--subscribe`: maps to `Mailbox.isSubscribed`

Notes:
- Servers may reject some changes based on ACLs; xin surfaces the standard SetError.

---

## 4) Write

**JSON schema (write):** SCHEMA.md §7

Write commands are defined by the RFCs (`urn:ietf:params:jmap:submission`).
Per the RFC-first principle, xin will send standard requests and surface any server errors as structured output.

### 4.0 `xin identities list|get <id>` (v0)
**JSON schema:** SCHEMA.md §7.1

- Lists available sending identities.
- Useful for figuring out which From addresses/aliases are permitted.

**JMAP:** `Identity/get`

### 4.1 `xin send --to ... --subject ... [--text <str|@file>] [--body-html <str|@file>] [--cc ...] [--bcc ...] [--attach <path>]... [--identity <id|email>]` (v0)
**gog analog:** `gog gmail send ...`
**JSON schema:** SCHEMA.md §7.2

Body input:
- `--text` accepts a literal string or `@/path/to/file.txt` to read from file.
- `--body-html` accepts a literal string or `@/path/to/file.html`.
- At least one of `--text`, `--body-html`, `--attach` must be provided.

Behavior (v0):
- Resolves the Drafts mailbox id (role=`drafts`).
- Resolves the sending Identity:
  - if `--identity` is provided, matches by Identity id or email.
  - otherwise uses the first Identity returned by `Identity/get`.
- Uploads each `--attach` via `uploadUrl` (RFC 8620 §6.1) to obtain `blobId`.
- Creates a draft via `Email/set` with a deterministic `bodyStructure`:
  - text+html → `multipart/alternative`
  - attachments → wrap in top-level `multipart/mixed` and append attachment parts (`blobId` + `name`).
- Submits via `EmailSubmission/set`.

### Error surfacing

- If a server rejects an upload or references an unknown blobId, xin reports the server’s standard error (e.g. `blobNotFound`) verbatim in structured output.

### 4.2 `xin drafts list|get|create|update|delete|send`
**gog analog:** `gog gmail drafts ...`
**JSON schema:** SCHEMA.md §7.3

Drafts are emails in the Drafts mailbox.

#### `xin drafts list [--max N] [--page TOKEN]` (v0)
- Implemented as `Email/query` with `inMailbox` set to the Drafts mailbox (resolved by role=`drafts` then name fallback).

#### `xin drafts get <draftEmailId> [--format full|metadata|raw]` (v0)
- Equivalent to `xin get`, but kept for parity/ergonomics.

#### `xin drafts create --to ... --subject ... (--body ... | --body-file ... | --body-html ...) [--cc ...] [--bcc ...] [--attach ...] [--identity <id|email>]` (v0)
- Uses `Email/set` create.
- MUST include membership of the Drafts mailbox.
- Body/attachments follow the exact same rules as `xin send` (uploadUrl + blobId; deterministic MIME layout).

#### `xin drafts update <draftEmailId> [--add ...] [--remove ...] [--add-mailbox ...] [--remove-mailbox ...] [--add-keyword ...] [--remove-keyword ...]` (v0)
- **Metadata-only** in-place update via `Email/set` `update`.
- Designed to **never change** the draft id.
- Auto routing rules:
  - `--add/--remove`: if the token resolves to a mailbox (by id/role/name), it is treated as mailbox; otherwise treated as a keyword.

#### `xin drafts rewrite <draftEmailId> [--to ...] [--cc ...] [--bcc ...] [--subject ...] [--body ...|--body-file ...] [--body-html ...] [--attach ...] [--replace-attachments] [--clear-attachments] [--identity <id|email>] [--destroy-old]` (v0)
- Rewrites message content by creating a **new** draft (`Email/set` create) and replacing the old one.
- Returns a new `draft.emailId` (id may change) and includes `replacedFrom`.
- Default cleanup is **non-destructive** (remove Drafts membership + unset `$draft`).
- `--destroy-old` permanently destroys the replaced draft, but requires global `--force`.
- Attachment behavior:
  - `--attach <path>` appends attachments by default.
  - `--replace-attachments` replaces existing attachments (requires at least one `--attach`).
  - `--clear-attachments` removes all attachments (cannot be combined with `--attach`).

#### `xin drafts delete <draftEmailId>...` (v0)
- **Non-destructive**: moves the Email out of the Drafts mailbox and into Trash (does not permanently delete the Email object).
- Also unsets the `$draft` keyword.

#### `xin drafts destroy <draftEmailId>...` (v0)
- **Destructive**: permanently deletes the Email object via `Email/set` destroy.
- Requires global `--force`.

#### `xin drafts send <draftEmailId> [--identity <id|email>]` (v0)
- Creates an `EmailSubmission` referencing the existing draft.

---

## 5) History / watch

### 5.1 `xin history [--since <state>] [--max N] [--page TOKEN] [--hydrate]`
**gog analog:** `gog gmail history`

- Purpose: **incremental sync cursor** for agents/automation.

Behavior:
- Bootstrap (no args): `xin history` returns the current Email collection state and empty changes.
- Incremental: `xin history --since <state>` uses JMAP `Email/changes` to return `created/updated/destroyed` **emailIds** since that state.
- Paging: when the server reports more changes, xin emits `meta.nextPage`; continue with `xin history --page <TOKEN>`.
- `--hydrate` (optional): also fetches summaries for `created` and `updated` ids via `Email/get` using JMAP backreferences, to reduce round-trips for agents.

Default: `--max 100`.

Output schema: see `docs/SCHEMA.md` §8.

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
