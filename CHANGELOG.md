# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3] - 2026-02-17


### Fixed
- Fastmail: if `xin reply` draft creation fails with `invalidProperties (header:In-Reply-To/References)` when using `header:*:asMessageIds`, retry with raw threading header text (`header:*:asText`) to ensure replies can still be sent.

## [0.1.2] - 2026-02-16

### Fixed
- Fix `xin reply` on Fastmail: set threading headers (`In-Reply-To`, `References`) via JMAP parsed header forms (`header:*:asMessageIds`) instead of raw header text, avoiding `invalidProperties (header:In-Reply-To)`.

### Tests
- Strengthen mock assertions for `reply` to validate the exact JMAP request shape.
- Add unit test to cover parsed message-id tokens and reference de-duplication.

## [0.1.1] - 2026-02-15

### Fixed
- Fix Fastmail strict capability handling for `Identity/*` (via jmap-client fork), avoiding `Unknown method` errors.
- Fix Fastmail send flow by avoiding setting the forbidden `$draft` keyword.

### Changed
- Prefer fixing provider quirks in the dependency (jmap-client fork) and keep xin-side code clean.

## [0.1.0] - 2026-02-15

Initial public release.

- Fastmail-first JMAP CLI
- JSON-first output contract with optional `--plain`
- Search/get/watch/history flows
- Basic send + reply helpers
