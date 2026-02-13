# History / watch — Implementation Notes

This note describes how xin implements incremental sync (`history`) and streaming change notification (`watch`).

## JMAP primitives

- `history` and `watch` are both built on **`Email/changes`** (RFC 8621).
- The server’s collection state string (`sinceState`/`newState`) is treated as an opaque cursor.

### history

- `xin history` (no args) bootstraps by fetching the **current Email collection state** (`Email/getState` equivalent via the client library) and returns an empty change set.
- `xin history --since <state>` calls `Email/changes` with `sinceState=<state>` and `maxChanges` (default 100).
- Paging:
  - When the server returns `hasMoreChanges=true`, xin emits `meta.nextPage`.
  - The page token encodes `sinceState` + `maxChanges` so callers don’t accidentally mix cursors.

### watch

`xin watch` is a polling loop around `Email/changes`.

Start cursor priority:
1) `--page`
2) `--checkpoint` (if file exists)
3) `--since`
4) current state bootstrap

Behavior:
- When there are changes, xin emits a `tick` event + one `email.change` event per id.
- When `--hydrate` is set, xin additionally fetches summaries of changed emails via `Email/get` and emits a single `email.hydrated` event.
- When there are no changes, xin sleeps for `intervalMs + jitterMs`.
- Ctrl-C is handled to stop quickly.

Checkpointing:
- `--checkpoint <FILE>` stores the latest page token after every successful poll.
- Write is best-effort atomic (write to `*.tmp` then rename).

## Local Stalwart harness gotcha (Linux CI)

The Stalwart docker harness bind-mounts `tests/feature/stalwart/.state/opt-stalwart` into the container.

On Linux CI, if the container runs as root, the host `.state` may become root-owned, which breaks `--fresh` (it needs to `rm -rf .state`).

Solution used by xin:
- Run the container as the host user via docker-compose `user: "${STALWART_UID}:${STALWART_GID}"`.
- Export these env vars in `tests/feature/stalwart/scripts/up.sh` using `id -u` / `id -g`.
