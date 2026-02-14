# SCHEMA.md vs source implementation (quick compare)

Goal: confirm the **stable JSON contract** in `docs/SCHEMA.md` matches what the CLI currently emits.

Compared:
- Contract doc: `references/SCHEMA.md` (copied from repo `docs/SCHEMA.md`)
- Implementation: `references/SCHEMA.from-source.md` (summary from `src/output.rs`, `src/schema.rs`, `src/commands/read.rs`)

Status: **looks consistent for v0.1** with a few small “doc is slightly ahead / slightly idealized” notes below.

---

## ✅ Matches (important)

- Top-level envelope fields and naming:
  - `schemaVersion` is `"0.1"`
  - `ok`, `command`, `account`, `data`, `error`, `meta`
  - `meta.nextPage`, `meta.warnings`, `meta.debug`

- Search item shape:
  - `threadId`, `emailId`, `receivedAt`, `subject`, `from`, `to`, `snippet`, `hasAttachment`, `mailboxIds`, `keywords`, `unread`

- `xin get` payload shape:
  - `data.email` contains `preview` (not `snippet`)
  - `data.body` includes `text/html` plus `textMeta/htmlMeta` in `--format full`
  - `data.attachments` objects include `{emailId, blobId, name, type, size, disposition}`
  - `data.raw` is only non-null for `--format raw`

- Conditional `email.headers`:
  - Source injects `email.headers` only when `--headers ...` is provided, matching contract notes.

- `thread get` default vs `--full`:
  - Default: `emails[]` are metadata objects
  - Full: `emails[]` are get-like objects with `email/body/attachments/raw`

---

## ⚠️ Minor “doc vs reality” notes

These are not bugs, just places where the contract doc is more prescriptive than the current data:

1) **Nullability / optional fields**
   - In source, `receivedAt` can be `null` if the provider/email lacks the timestamp.
   - Several header/body/preview fields can be `null`.
   - The contract doc mostly shows non-null examples (fine), but agents should handle nulls.

2) **`get --format metadata` body/attachments**
   - Source returns `body: {text:null, html:null}` and `attachments: []`.
   - The contract doc describes “structure pointers” (as a goal). That’s fine, but agents should assume metadata mode does not include body/attachments bytes.

3) **`meta.debug` stability**
   - Contract explicitly says debug is not stable; source matches.

If you want, we can tighten `docs/SCHEMA.md` wording to explicitly call out nullability and the current metadata-mode body/attachments behavior.
