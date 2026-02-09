# Implementation: MAILBOXES (labels/mailboxes)

Covers:
- `xin labels/mailboxes list|get|create|modify|rename|delete`

References:
- RFC 8621 §2 (Mailboxes)
- RFC 8620 §5.3 (/set)

---

## 1) List / get

- List: `Mailbox/get` with `ids: null`
- Get (v0):
  - If the input looks like a mailbox id (base64url-ish), xin MAY try `Mailbox/get` with `ids: [<id>]` first.
  - Otherwise xin performs **list+resolve**:
    1) `Mailbox/get` with `ids: null`
    2) resolve `<mailboxId|name|role>` to a concrete id (see `docs/CLI.md`)
    3) return the matching mailbox object

Notes:
- This keeps common human usage (`inbox`, `trash`, or mailbox names) simple, while still supporting fast id lookups when available.

---

## 2) Create

Use `Mailbox/set` create.

Example:

```json
{
  "using": ["urn:ietf:params:jmap:core","urn:ietf:params:jmap:mail"],
  "methodCalls": [
    ["Mailbox/set", {
      "accountId": "A",
      "create": {
        "k1": {
          "name": "MyFolder",
          "parentId": null,
          "isSubscribed": true
        }
      }
    }, "s1"]
  ]
}
```

Normalize to `SCHEMA.md §5.3` (`created/updated/destroyed`).

---

## 3) Modify / rename

Use `Mailbox/set` update. For rename, update only `name`.

```json
{
  "using": ["urn:ietf:params:jmap:core","urn:ietf:params:jmap:mail"],
  "methodCalls": [
    ["Mailbox/set", {
      "accountId": "A",
      "update": {
        "mbx123": { "name": "NewName" }
      }
    }, "s1"]
  ]
}
```

---

## 4) Delete

Use `Mailbox/set` destroy.

- Without `--remove-emails`: keep default `onDestroyRemoveEmails=false`.
- With `--remove-emails`: set `onDestroyRemoveEmails=true`.

```json
{
  "using": ["urn:ietf:params:jmap:core","urn:ietf:params:jmap:mail"],
  "methodCalls": [
    ["Mailbox/set", {
      "accountId": "A",
      "onDestroyRemoveEmails": true,
      "destroy": ["mbx123"]
    }, "s1"]
  ]
}
```
