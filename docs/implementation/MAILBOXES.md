# Implementation: MAILBOXES (labels/mailboxes)

Covers:
- `xin labels/mailboxes list|get|create|modify|rename|delete`

References:
- RFC 8621 §2 (Mailboxes)
- RFC 8620 §5.3 (/set)

---

## 1) List / get

- List: `Mailbox/get` with `ids: null`
- Get: `Mailbox/get` with `ids: [<id>]`

Role/name resolution is defined in `docs/CLI.md`.

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
