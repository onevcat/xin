# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
