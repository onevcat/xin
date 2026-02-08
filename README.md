# xin (信)

> Agent-first multi-mailbox CLI.
>
> Goal: keep using `gog` for Gmail, use JMAP for Fastmail/other JMAP providers, and provide a single interface for agents/humans to **check, summarize, archive, read, reply, send**.

## Why

- Gmail automation is great via `gog`.
- Fastmail (custom domain) has strong JMAP support.
- We want one command to check **all** inboxes and output a unified summary (JSON-first).

## Principles

- **Agent-first**: stable JSON output by default; human-friendly output via `--pretty`.
- **Safe by default**: read-only unless `--yes` / `--force` (or explicit action commands).
- **Provider adapters**: Gmail adapter wraps `gog`; JMAP adapter uses a maintained JMAP library.
- **Unified core actions**: read / archive / reply / send (optional trash).

## Docs

- [Initial spec: gog parity + Gmail vs JMAP analysis](docs/INITIAL.md)
