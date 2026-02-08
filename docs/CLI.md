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

### 1.1 `xin search <query> [--max N] [--page TOKEN] [--oldest]`
**gog analog:** `gog gmail search` (threads)

- Returns: list of threads (or thread-like groups), with summary fields.

Query support (DSL v0):
- We intentionally support a **Gmail-inspired subset** of operators for muscle memory, but this is **xin’s own DSL**.
- Implementation-wise, this means: **parse DSL → build a JMAP `Email/query` filter JSON**.
- Always offer an escape hatch:
  - `--filter-json '<JMAP Email/query filter>'` (raw JMAP)

Why “Gmail-inspired”?
- Gmail’s search operators are publicly documented (user-facing, not a formal grammar):
  - https://support.google.com/mail/answer/7190

#### Supported operators (initial)

- Addressing:
  - `from:<addr|text>`
  - `to:<addr|text>`
  - `cc:<addr|text>`
  - `bcc:<addr|text>`
- Content:
  - `subject:<text>`
  - `"exact phrase"` (quoted)
  - bare terms: `foo bar` (free text)
- Time:
  - `after:YYYY/MM/DD`  (or `YYYY-MM-DD`)
  - `before:YYYY/MM/DD`
  - `newer_than:<Nd|Nm|Ny>`
  - `older_than:<Nd|Nm|Ny>`
- State:
  - `is:unread` / `is:read`  (maps to keyword `$seen`)
  - `is:starred`             (maps to keyword `$flagged`)
- Location:
  - `in:inbox` / `in:archive` / `in:trash` / `in:spam` (provider permitting)
  - `in:anywhere` (no mailbox restriction)
  - `label:<name>` is accepted as an alias of `in:<name>` for familiarity (JMAP has mailboxes, not Gmail labels)
- Attachments:
  - `has:attachment`

#### Boolean logic (initial)

- AND: implicit by whitespace
- NOT: prefix `-` (e.g. `-from:foo@bar.com`, `-subject:"weekly"`)

**TBD:** `OR`, parentheses, `{}` groups, `AROUND`, size operators (`larger:`/`smaller:`), filename/type operators (`filename:`), category operators (`category:`).

#### Compilation to JMAP filters (high-level)

xin compiles the DSL into a JMAP `Email/query` filter.

- `from:` → `filter.from = "..."`
- `to:`   → `filter.to = "..."`
- `cc:` / `bcc:` → corresponding fields
- `subject:` → `filter.subject = "..."`
- bare terms / quoted phrases → `filter.text = "..."` (preferred)
- `has:attachment` → `filter.hasAttachment = true`
- `after:` / `before:` / `newer_than:` / `older_than:` → `filter.after` / `filter.before` (date math)
- `is:unread` → `filter.notKeyword = "$seen"`
- `is:read`   → `filter.hasKeyword = "$seen"`
- `is:starred` → `filter.hasKeyword = "$flagged"`
- `in:<mailbox>` → `filter.inMailbox = <mailboxId>` (resolved by name via `Mailbox/get`)
- NOT (`-term`) → compiled using JMAP’s `operator: "NOT"` wrapper (exact structure TBD)

Note: the exact JMAP filter field names/structures are per RFC 8621 / JMAP Mail spec; xin should validate against server capabilities at runtime.

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
