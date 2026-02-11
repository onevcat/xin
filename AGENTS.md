# Repository Guidelines

## Project Structure & Module Organization
`xin` is a Rust CLI for JMAP. Core code lives in `src/`:
- `src/main.rs` bootstraps CLI parsing and command dispatch.
- `src/cli.rs` defines command/flag contracts.
- `src/commands/` contains command handlers (`read`, `organize`, `labels`, `send`, `inbox`).
- Shared runtime pieces are in `src/config.rs`, `src/jmap.rs`, `src/output.rs`, and `src/schema.rs`.

Integration tests are in `tests/*.rs` (wiremock-based). Real-server feature tests are under `tests/feature/stalwart/` with declarative cases in `tests/feature/stalwart/cases/`. Specs and implementation notes are in `docs/` (`CLI.md`, `SCHEMA.md`, `implementation/*.md`).

## Build, Test, and Development Commands
- `cargo build` builds the CLI (`target/debug/xin`).
- `cargo run -- --help` runs the local binary and prints command usage.
- `cargo test` runs unit + integration tests.
- `cargo test --test read_search_mock` runs a single integration test file.
- `cd tests/feature/stalwart && ./scripts/up.sh && ./scripts/seed.sh && ./scripts/check.sh` starts local Stalwart, seeds accounts, and runs a connectivity check.
- `cargo run --bin xin-feature -- --fresh --case-dir tests/feature/stalwart/cases --all` runs declarative feature cases.

## Coding Style & Naming Conventions
Follow idiomatic Rust with default `rustfmt` formatting (4-space indentation). Use `snake_case` for files, modules, and functions; `CamelCase` for types/enums; `SCREAMING_SNAKE_CASE` for constants/env keys. Keep command behavior JSON-first and stable; when adding flags or output fields, update `docs/CLI.md` and `docs/SCHEMA.md` in the same change.

## Testing Guidelines
Prefer integration tests for user-visible CLI behavior:
- Place tests in `tests/*_mock.rs` and assert envelope fields (`ok`, `command`, `data`, `meta`).
- Use `wiremock` for JMAP API interactions and `assert_cmd` for CLI execution.
- Add/adjust Stalwart feature YAML cases when behavior spans multiple commands or server states.

No fixed coverage threshold exists, but every new command path and regression fix should include at least one test.

## Commit & Pull Request Guidelines
Commit messages follow Conventional Commit style seen in history: `feat(scope): ...`, `fix(scope): ...`, `test(scope): ...`, `docs: ...`, `perf(scope): ...`.

PRs should include:
- Clear problem/solution summary and impacted commands.
- Linked issue (if any) and migration notes for breaking CLI/schema changes.
- Updated docs (`docs/CLI.md`, `docs/SCHEMA.md`) and relevant tests.
- Example invocation/output when changing user-facing JSON or flags.

## Security & Configuration Tips
Do not commit real credentials. Use env vars (`XIN_BASE_URL`, `XIN_TOKEN` / `XIN_BASIC_USER` + `XIN_BASIC_PASS`) and local-only test credentials in `tests/feature/stalwart`.
