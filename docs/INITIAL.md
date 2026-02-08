# xin (信) — Initial Spec

This document:

1) Surveys `gog gmail` command surface (what exists today for Gmail)
2) Proposes a `xin` command surface that maps 1:1 (where sensible)
3) Analyzes Gmail vs JMAP API differences: gaps, and opportunities to exceed `gog`/Gmail

> Scope note: xin’s goal is **not** to replace a full email client UI.
> It is an automation CLI with excellent defaults for agents.

---

## 0. Terminology

- **Gmail adapter**: uses `gog` underneath.
- **JMAP adapter**: talks to JMAP servers (Fastmail first), via a maintained JMAP client library.
- **Normalized objects**: xin returns a unified schema (Message/Thread/Mailbox/Label) regardless of provider.

---

## 1) `gog gmail` command surface (survey)

`gog` provides these Gmail top-level commands (v0.9.0).

Common flags worth mirroring in xin:

- Output:
  - `--json`: JSON to stdout (script/agent friendly)
  - `--plain`: stable TSV output (no colors)
- Safety:
  - `--force`: skip confirmations for destructive commands
  - `--no-input`: never prompt; fail instead
- Auth/account selection:
  - `--account <email>`
  - `--client <oauth-client-name>`

### Read
- `gog gmail search <query> --max N [--page TOKEN] [--oldest]`
  - Search **threads** using Gmail query syntax.
- `gog gmail messages search <query> --max N`
  - Search **messages** (per-email, ignoring threading).
- `gog gmail get <messageId> [--format full|metadata|raw] [--headers ...]`
- `gog gmail thread get <threadId> [--download] [--full] [--out-dir ...]`
- `gog gmail thread attachments <threadId>`
- `gog gmail attachment <messageId> <attachmentId> [--out ...] [--name ...]`
- `gog gmail url <threadId>...`
- `gog gmail history [--since HISTORY_ID] [--max N] [--page TOKEN]`

### Organize
- `gog gmail thread modify <threadId> --add LABELS --remove LABELS`
- `gog gmail batch modify <messageId>... --add LABELS --remove LABELS`
- `gog gmail batch delete <messageId>...` (permanent delete)

### Labels
- `gog gmail labels list|get|create`
- `gog gmail labels modify <threadId>... --add ... --remove ...`

### Write
- `gog gmail send --to ... --subject ... --body/--body-file/--body-html ... [--attach ...] [--thread-id ...] [--reply-to-message-id ...] [--reply-all] ...`
- `gog gmail drafts list|get|create|update|delete|send`

### Tracking (gog feature)
- `gog gmail track setup|opens|status`

### Settings/Admin (Gmail-specific)
- `gog gmail settings filters list|get|create|delete`
- `gog gmail settings watch start|status|renew|stop|serve`
- `gog gmail settings delegates list|get|add|remove`
- `gog gmail settings forwarding list|get|create|delete`
- `gog gmail settings autoforward get|update`
- `gog gmail settings sendas list|get|create|verify|delete|update`
- `gog gmail settings vacation get|update`

Notes:
- These are largely **Gmail-only admin surfaces**. xin should support them under `xin gmail ...` but not attempt to standardize them across providers in v1.
- Portable “rules” should be client-side (xin rules engine) rather than server-side filters.

---

## 2) Proposed `xin` CLI surface (agent-first)

### 2.1 Core: provider-agnostic commands

These are the commands an agent/human uses day-to-day:

- `xin check`  
  Summarize all configured accounts (Gmail + JMAP). Output normalized JSON by default.

- `xin inbox list [--account <name>|--all] [--unread] [--max N]`
- `xin thread list/get <id>`
- `xin message get <id>`

- `xin archive <id>...`  
  Default behavior: remove from inbox (never delete).

- `xin read <id>...`  
  Marks as read.

- `xin reply <id> --body-file ... [--reply-all] [--attach ...]`
- `xin send --to ... --subject ... --body-file ... [--attach ...]`

- `xin trash <id>...` (optional; requires `--yes`)

- `xin rules test --rules rules.yaml [--max N]`  
  Explain classification (must-do vs archive) without making changes.

### 2.2 Provider-specific escape hatches (parity with gog)

Because Gmail and JMAP differ, xin should support:

- `xin gmail ...` : a thin wrapper that maps to `gog gmail ...` semantics
- `xin jmap ...`  : direct JMAP method-level calls for power users (JSON in/out)

This keeps xin useful for *all* providers while retaining deep capability when needed.

---

## 3) Mapping: `gog gmail` → `xin`

This section is intentionally “1:1-ish” with `gog gmail`, so we don’t lose power.

| `gog gmail` | What it does | `xin` equivalent | Notes |
|---|---|---|---|
| `search` (threads) | Gmail query on threads | `xin search --gmail-query ...` or `xin inbox list --filter ...` | Gmail query syntax is Gmail-only; xin may support both `--gmail-query` and normalized filters |
| `messages search` | Gmail query on messages | `xin search --scope message --gmail-query ...` | |
| `get <messageId>` | Fetch message | `xin message get <id>` | Normalized schema; provider raw via `--raw` |
| `thread get <threadId>` | Fetch full thread | `xin thread get <id>` | JMAP Thread support varies; fallback to grouping by headers |
| `attachment` | Download attachment | `xin attachment get <messageId> <attachmentId>` | JMAP uses blob/download URLs |
| `url` | Gmail web URL | `xin open <id>` | For JMAP providers, webmail URL may be unavailable |
| `history` | Gmail incremental history | `xin sync status` / `xin events` | JMAP has `*/changes` APIs; can approximate |
| `thread modify --add/--remove` | Label changes on thread | `xin tag add/remove <threadId>` / `xin archive` | Gmail labels vs JMAP keywords/mailboxes |
| `batch modify` | Label changes on messages | `xin tag add/remove <messageId>...` | |
| `batch delete` | Permanent delete | `xin delete` (discouraged) | Prefer `trash` |
| `labels list/get/create` | Gmail labels | `xin labels list/get/create` | For JMAP: Mailboxes + keywords; map to a unified concept |
| `send` | Send mail | `xin send` | JMAP send is draft+submission; Gmail uses Gmail API |
| `drafts ...` | Draft ops | `xin drafts ...` | JMAP has draft mailbox; standardize |
| `track ...` | Open tracking (gog feature) | `xin track ...` (optional) | Provider-independent feature; xin could exceed here |
| `settings filters` | Manage Gmail filters | `xin gmail filters ...` | No portable equivalent; JMAP servers may expose Sieve via other protocols |
| `settings watch ...` | Gmail Pub/Sub watch | `xin watch ...` | JMAP has WebSocket push (RFC 8887) on some servers; can exceed portability |

---

## 4) Gmail API vs JMAP: differences, gaps, opportunities

### 4.1 Data model

**Gmail**
- First-class: `Thread`, `Message`, **Labels** (INBOX, UNREAD, STARRED, etc)
- Strong Gmail-only features: Categories (Promotions/Updates), “importance”, rich web URLs

**JMAP**
- First-class: `Email`, `Mailbox`, `Thread` (standard but server support varies), `EmailSubmission`
- Tags are usually **keywords** (e.g. `$seen`, `$flagged`), plus mailbox membership

Implications for xin:
- Provide normalized actions: **archive/read/trash** that map correctly per provider.
- Provide provider-specific query modes when necessary.

### 4.2 Query/search

- Gmail: powerful free-form query language (`from:`, `to:`, `newer_than:`, etc)
- JMAP: structured filters (JSON). Some servers provide full-text search, but features vary.

Xin approach:
- Support a small **portable filter DSL** (from/to/subject/hasAttachment/unread/inbox) 
- Plus `--gmail-query` escape hatch; plus `--jmap-filter-json` for power users

### 4.3 Threads

- Gmail threads are always present.
- JMAP threading depends on server implementation and/or how it groups; may differ.

Xin approach:
- Keep both `messageId` and `threadId` in normalized schema.
- If JMAP Thread is missing/unreliable, approximate using `Message-ID`, `In-Reply-To`, `References`.

### 4.4 Incremental sync / events

- Gmail: `historyId` + watch via Pub/Sub.
- JMAP: `*/changes` and (optionally) push via WebSocket (RFC 8887) on supporting servers.

Opportunities:
- xin can implement **portable polling** + optionally **push** where available.
- This can become *better than gog* for multi-provider “check all inboxes” workflows.

### 4.5 Server-side rules

- Gmail: filters/settings management via Gmail API (gog exposes some).
- JMAP: no universal “filters” spec; providers may support Sieve/ManageSieve or proprietary endpoints.

Xin stance:
- Don’t try to standardize server-side filtering initially.
- Instead: provide a powerful **client-side rules engine** (classify + actions) that works everywhere.

### 4.6 Attachments

- Gmail: attachmentId per part; download via Gmail API.
- JMAP: blobs/download URLs; can be simpler once normalized.

### 4.7 Things we likely *cannot* portably implement

- Gmail Categories/Smart features (promotions/updates importance) in a portable way.
- Gmail-specific settings admin surfaces (delegates, send-as settings, vacation responder) across all providers.

We can still support them as `xin gmail ...` subcommands.

### 4.8 Things xin can plausibly *exceed*

- Multi-account, multi-provider unified checking and summarization.
- Deterministic, stable JSON output designed for LLM/agent consumption.
- A rules engine that works for both Gmail and JMAP providers.
- Optional portability of “watch” via polling + JMAP push where supported.
- Provider-independent open tracking (similar to gog’s, but generalized).

---

## 5) MVP checklist (to validate end-to-end)

1) Gmail adapter: wrap `gog gmail search` and normalize output
2) JMAP adapter (Fastmail): list inbox/unread, fetch subject/from/date/snippet
3) `xin check`: merges and ranks “must handle” items (Jira/GitHub/UWV)
4) `xin archive`: safe default (remove from inbox), `--dry-run` support
5) Rules config file (`rules.yaml`) for must-do classification

---

## 6) Open questions

- Normalized ID format: e.g. `gmail:thread:19c3...` vs `jmap:email:abc...`
- Where to store credentials/config (OS keychain vs file + env var)
- Library choice for JMAP (Go vs Rust) based on licensing/coverage/maintenance
