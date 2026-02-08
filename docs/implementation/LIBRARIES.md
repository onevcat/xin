# JMAP client library survey for xin

This document surveys popular JMAP client libraries and evaluates them against xin’s needs.

xin needs (v0):

- JMAP Core (RFC 8620): Session, API requests (methodCalls), result references, `/query` paging, uploadUrl/downloadUrl
- JMAP Mail (RFC 8621): `Mailbox/*`, `Email/query|get|set`, `Thread/get`, `SearchSnippet/get` (optional)
- JMAP Submission (RFC 8621): `Identity/get`, `EmailSubmission/set`
- Good ergonomics for building **multi-method** requests
- Permissive license (preferred)
- Active maintenance

---

## Shortlist (recommended)

### 1) stalwartlabs/jmap-client (Rust)

- URL: https://github.com/stalwartlabs/jmap-client
- Language: Rust
- License: Apache-2.0 OR MIT (dual)
- Activity: active releases (e.g. v0.4.0), recent updates
- Popularity: moderate (~100 stars)

Coverage (per README):
- RFC 8620 (core) ✅
- RFC 8621 (mail) ✅
- RFC 8887 (WebSocket) ✅
- EventSource ✅
- Helpers/builders for filters, requests, method references ✅
- Blob upload/download helpers ✅ (see `blob` module)

Pros:
- Very complete and RFC-aligned; strong typed API; good docs/examples.
- Includes both blocking + async, and WebSocket push (future-proof).
- Dual permissive license.

Cons / watchouts:
- Rust integration (fine if we choose Rust for xin).
- Some APIs are higher-level convenience; xin must still control exact request shapes when needed.

Fit for xin: **Excellent**.

#### Fastmail smoke test notes (stalwartlabs/jmap-client) — 2026-02-08

A quick local smoke test was run against Fastmail using a dedicated API token, using a throwaway Rust harness under `/tmp` (not committed).

What worked (read-only / verification):
- Session connect via `https://api.fastmail.com` ✅
- `Mailbox/query` resolving Inbox by `role=inbox` ✅
- Single request: `Email/query` (`collapseThreads=true`, `receivedAt desc`, `limit=10`) + result reference → `Email/get` ✅
- `Email/get` (headers/preview fields) ✅
- Single request: `Thread/get` + result reference → `Email/get` ✅
- `Identity/get` (submission capability) ✅
- Blob download for an attachment (`download(blobId)`) ✅
- Blob upload via `uploadUrl` (unreferenced test blob) ✅

One small gotcha:
- Fastmail may redirect the session URL, and the client refuses redirects to hosts that are not explicitly trusted.
  - Fix: configure `follow_redirects(["api.fastmail.com", "jmap.fastmail.com", "fastmail.com", "www.fastmail.com"])` (exact allowlist TBD).
  - For xin: treat this as a **policy** decision (e.g. default trust same-host, optionally allow additional hosts), not a provider-specific hack.

### 2) ~rockorager/go-jmap (Go)

- URL: https://sr.ht/~rockorager/go-jmap
- Language: Go
- License: MIT
- Activity: active (recent commits in 2025)
- Popularity: sourcehut project (no GitHub stars signal)

Coverage (per README + code scan):
- RFC 8620 (core) ✅ (Request/Invoke, result references, EventSource, PushSubscription)
- RFC 8621 (mail) ✅
- Submission (`urn:ietf:params:jmap:submission`) ✅ (Identity/*, EmailSubmission/* present)
- uploadUrl/downloadUrl ✅ (`Client.Upload`, Session includes both)
- RFC 8887 (WebSocket) ❌ (not started)

Pros:
- Good low-level building blocks; easy to construct RFC-shaped requests.
- Go distribution story for CLI is excellent.
- MIT license.

Cons / watchouts:
- No WebSocket support (not required for v0).
- API is more “protocol-level”; xin still needs more glue code.

Fit for xin: **Excellent**.

---

## Other libraries (not recommended as primary for xin)

### meli/melib (Rust)
- URL: https://meli.delivery / https://git.meli-email.org
- License: GPL-3.0
- Activity: very active
- Reason not primary: GPL licensing + library is oriented around a full mail client stack.

### jmap-rs (Rust)
- URL: https://gitlab.com/jmap-rs/jmap-rs
- License: MPL-2.0
- Status: README indicates unfinished; last activity appears old.
- Reason not primary: maturity risk.

### jmap-js (JavaScript)
- URL: https://github.com/jmapio/jmap-js
- License: MIT
- Status: mature but last core commit in 2020; heavier “offline-first model” library.
- Reason not primary: not Go/Rust; also bigger abstraction than we need.

### jmap-client-ts (TypeScript)
- URL: https://github.com/OpenPaaS-Suite/jmap-client-ts
- License: MIT
- Status: actively updated
- Reason not primary: language mismatch if we choose Go/Rust for xin.

### jmap-jam (TypeScript)
- URL: https://github.com/htunnicliff/jmap-jam
- License: MIT
- Status: active; very ergonomic; supports upload/download + references.
- Reason not primary: language mismatch (but it’s a great reference design).

### jmap-yacl (TypeScript)
- URL: https://github.com/ilyhalight/jmap-yacl
- License: MIT
- Status: active; explicitly “basic auth only”; “Push not implemented”.
- Reason not primary: language mismatch + narrower scope.

### jmapc (Python)
- URL: https://github.com/smkent/jmapc
- License: GPL-3.0
- Status: good coverage, tests, EventSource
- Reason not primary: GPL + language mismatch.

---

## Recommendation

For xin (JMAP-only RFC-first CLI), the top two candidates are:

1) **Rust: stalwartlabs/jmap-client** — best overall completeness (core+mail+submission+push), excellent docs.
2) **Go: ~rockorager/go-jmap** — excellent protocol-level client with submission + upload, great for a Go CLI.

Next step (selection):

- Pick language based on preferred ecosystem for the CLI implementation.
- Run a 30-minute smoke test using each library to implement:
  - Session fetch
  - `Mailbox/get ids=null`
  - `Email/query + Email/get` with backreference
  - uploadUrl upload (dummy small blob)

The one that feels cleaner for these four tasks is the winner.
