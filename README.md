# xin（信）

> Agent-first JMAP CLI.
>
> **Design goal:** `xin` is to JMAP email what `gog gmail` is to Gmail.
> It is **not** a multi-provider wrapper and **not** a replacement for `gog`.

## What it is

- A command-line tool that talks to **JMAP servers** (Fastmail first; any standard JMAP server later)
- Command surface intentionally **mirrors** `gog gmail` for usability and muscle memory
- Output is **JSON-first** and stable for agents

## What it is not

- Not a Gmail tool
- Not a unified “check all inboxes” tool (that can be a separate orchestrator later)

## Docs

- [Initial spec (overview)](docs/INITIAL.md)
- [CLI contract (full flags/commands)](docs/CLI.md)
- [JSON output schema (agent-first contract)](docs/SCHEMA.md)
