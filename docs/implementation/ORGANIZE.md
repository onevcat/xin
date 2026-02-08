# Implementation: ORGANIZE

Covers:

- `xin thread modify` / `xin batch modify`
- sugar commands:
  - `xin archive/read/unread/trash`
  - `xin thread archive/read/unread/trash`
  - email-level `--whole-thread`

References:
- RFC 8621 §4.6 `Email/set`
- RFC 8621 §3.1 `Thread/get`

---

## 0) Mailbox resolution helper (role/name → mailboxId)

Organize commands need mailbox ids (inbox/trash/junk/etc). Use `Mailbox/get` with `ids: null` to fetch all mailboxes, then build:

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

Notes:
- Role/name resolution order is defined in `docs/CLI.md`.
- Mailbox command implementations live in `MAILBOXES.md` (this section is just the shared helper).

---

## 1) Email-level modify (keywords/mailboxIds)

Use `Email/set` update with patch syntax (RFC 8620 §5.3).

Example: mark read (`$seen`):

```json
{
  "using": ["urn:ietf:params:jmap:core","urn:ietf:params:jmap:mail"],
  "methodCalls": [
    ["Email/set", {
      "accountId": "A",
      "update": {
        "M123": {
          "keywords/$seen": true
        }
      }
    }, "s1"]
  ]
}
```

Example: remove inbox membership (archive-style):

```json
{
  "using": ["urn:ietf:params:jmap:core","urn:ietf:params:jmap:mail"],
  "methodCalls": [
    ["Email/set", {
      "accountId": "A",
      "update": {
        "M123": {
          "mailboxIds/<inboxMailboxId>": null
        }
      }
    }, "s1"]
  ]
}
```

Example: trash (set mailboxIds to only trash mailbox):

- First fetch existing mailboxIds? Not required if we use full replacement.
- Use full `mailboxIds` replacement to `{ trashId: true }`.

```json
{
  "using": ["urn:ietf:params:jmap:core","urn:ietf:params:jmap:mail"],
  "methodCalls": [
    ["Email/set", {
      "accountId": "A",
      "update": {
        "M123": {
          "mailboxIds": {"<trashMailboxId>": true}
        }
      }
    }, "s1"]
  ]
}
```

---

## 2) Thread-level operations

Thread-level commands must expand via `Thread/get` (RFC-compliant), then apply `Email/set` to each emailId.

Example: `xin thread read T123`:

```json
{
  "using": ["urn:ietf:params:jmap:core","urn:ietf:params:jmap:mail"],
  "methodCalls": [
    ["Thread/get", {"accountId":"A", "ids":["T123"]}, "t1"],
    ["Email/set", {
      "accountId": "A",
      "update": {
        "#t1/list/0/emailIds/*": { "keywords/$seen": true }
      }
    }, "s1"]
  ]
}
```

Note:
- The above uses a conceptual "apply to each" notation; actual JMAP requires an `update` object keyed by explicit ids.
- Implementation should build the `update` map client-side from the returned `emailIds`.

---

## 3) `--whole-thread` (email-level flag)

Implementation:

1) `Email/get` the given emailId and read `threadId`
2) perform the corresponding thread-level action

This keeps the CLI explicit while remaining convenient.

---

## 4) `--dry-run`

- xin should compute the exact `Email/set` patch it *would* send.
- When `--dry-run` is set, do not send the `Email/set`; return:
  - `dryRun: true`
  - `changes`: intended mailbox/keyword diffs
  - `appliedTo`: computed targets (expanded emailIds if needed)
