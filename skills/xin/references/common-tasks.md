# xin Common Tasks

This doc is a curated, **workflow-first** guide for using `xin` to help a human manage JMAP mail (Fastmail-first).

If you need the full flag/option reference, use:
- [commands](./commands.md)
- Per-command docs under `./*.md`
- JSON contract: [SCHEMA.md](./SCHEMA.md)

---

## 0) Setup / sanity check

```bash
xin config init
xin auth set-token <FASTMAIL_API_TOKEN>

# Quick ping: can we query Inbox?
xin search "in:inbox" --max 1
```

Tips:
- For humans: add `--plain`.
- For agents/scripting: keep default `--json`.

---

## 1) Inbox triage (Inbox Zero loop)

### List unread Inbox

```bash
xin --plain messages search "in:inbox seen:false" --max 50
# or JSON
xin messages search "in:inbox seen:false" --max 50
```

### Process one by one (recommended)

```bash
xin inbox next
xin inbox do <emailId> read
xin inbox do <emailId> archive
# or
xin inbox do <emailId> trash

# Apply to the whole thread:
xin inbox do <emailId> archive --whole-thread
```

Why this loop works well:
- `inbox next` gives you the next actionable item.
- `inbox do` keeps the action vocabulary small and consistent.

---

## 2) Read an email (details)

```bash
# Fast metadata (headers-ish)
xin --plain get <emailId> --format metadata

# Full content (may truncate huge bodies; see meta.warnings)
xin --plain get <emailId> --format full

# JSON full
xin get <emailId> --format full
```

Thread view:

```bash
xin thread get <threadId>
xin thread get <threadId> --full
```

---

## 3) Search patterns (query sugar)

```bash
xin search "from:github" --max 10
xin search "subject:invoice" --max 10
xin search "in:inbox seen:false" --max 50
xin search "has:attachment" --max 20
xin search "after:2026-01-01 before:2026-01-31" --max 20

# OR
xin search "or:(from:github | from:atlassian) seen:false" --max 20

# NOT (single-term)
xin search "-in:Trash from:newsletter" --max 50
```

Advanced / exact: native JMAP filter JSON

```bash
xin search --filter-json '{"text":"hello"}' --max 10
# file input also supported:
xin search --filter-json @filter.json --max 10
```

---

## 4) Batch organize (jq + xargs)

### Archive everything returned by a search

```bash
xin messages search "in:inbox from:newsletter" --max 200 --json \
  | jq -r '.data.items[].emailId' \
  | xargs -n 50 sh -c 'xin batch modify "$@" --remove inbox --add archive' _
```

### Mark many as read

```bash
xin messages search "in:inbox seen:false" --max 200 --json \
  | jq -r '.data.items[].emailId' \
  | xargs -n 50 sh -c 'xin batch modify "$@" --add $seen' _
```

Always consider `--dry-run` first:

```bash
xin batch modify <emailId> --remove inbox --add archive --dry-run
```

---

## 5) Send / reply workflows

### Send a simple text email

```bash
xin send --to someone@example.com --subject "Hello" --text "Hi"
```

### Prefer file input for multi-line content

```bash
cat > /tmp/body.txt <<'EOF'
Hi,

Here is the update.

- Item 1
- Item 2
EOF

xin send --to someone@example.com --subject "Update" --text @/tmp/body.txt
```

### “Reply” (v0 minimal: new message)

Threading headers are not wired yet, so the safest flow is:
- Use `xin get` to extract sender + subject
- Use `xin send` to send a new email
- Or use Fastmail UI via `xin url <emailId>` for a real threaded reply

```bash
xin url <emailId>   # Fastmail-only
```

---

## 6) Drafts

```bash
xin drafts list --max 20
xin drafts get <draftEmailId> --format full
xin drafts create --to someone@example.com --subject "Draft" --text "..."
xin drafts send <draftEmailId>

# Delete (non-destructive): move out of Drafts and into Trash
xin drafts delete <draftEmailId>

# Destroy (destructive): requires --force
xin --force drafts destroy <draftEmailId>
```

---

## 7) Labels / mailboxes

```bash
xin labels list
xin labels get inbox
xin labels create "Projects"

# Apply/remove mailbox/keyword with batch modify
xin batch modify <emailId> --add "Projects"
xin batch modify <emailId> --remove "Projects"
```

Note: in JMAP, “labels” are mailboxes.

---

## 8) Automation: history / watch

### history (incremental sync)

```bash
xin history
xin history --since <state>
```

### watch (polling stream)

```bash
xin watch --checkpoint /tmp/xin.watch.token
# human-friendly
xin --plain watch --checkpoint /tmp/xin.watch.token
```

---

## 9) jq patterns (using the stable JSON contract)

```bash
# Extract IDs
xin search "in:inbox" --max 50 --json | jq -r '.data.items[].emailId'

# Show subject + from
xin search "in:inbox" --max 20 --json \
  | jq -r '.data.items[] | [.receivedAt, (.from[0].email // ""), (.subject // "")] | @tsv'

# Check errors
xin search "in:inbox" --json | jq '.ok, .error'

# Pagination token
xin search "in:inbox" --max 100 --json | jq -r '.meta.nextPage'
```
