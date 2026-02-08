# xin — Implementation Notes (RFC-first)

This folder contains implementation-oriented notes and examples for building xin.

- CLI contract: `docs/CLI.md`
- JSON output contract: `docs/SCHEMA.md`

Principles:

- **RFC-first**: construct standard JMAP requests per RFC 8620 (core) and RFC 8621 (mail).
- **No provider workarounds**: xin sends the standard request and surfaces server errors.
- **One feature at a time**: implement Read-only MVP → Organize → Write.

Conventions used in examples:

- `SESSION_URL`: user-provided JMAP session endpoint (e.g. `https://api.fastmail.com/.well-known/jmap`).
- `apiUrl`, `uploadUrl`, `downloadUrl`: obtained from Session.
- `accountId`: obtained from Session.
- Requests are shown as the JMAP **Request object** body (HTTP POST to `apiUrl`).
- `using`: must include the relevant capabilities (`urn:ietf:params:jmap:core`, `urn:ietf:params:jmap:mail`, and for sending `urn:ietf:params:jmap:submission`).

Files:

- `READ.md`: Session, search, get, thread get, attachment list/download.
- `ORGANIZE.md`: archive/read/trash and modify operations (Email/set patches), thread expansion.
- `MAILBOXES.md`: labels/mailboxes (Mailbox/get|set).
- `WRITE.md`: identities, uploadUrl blobs, Email/set draft creation, EmailSubmission/set send.
