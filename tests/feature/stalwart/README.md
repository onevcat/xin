# Stalwart (Docker) – feature test seed (Phase A)

This directory hosts the local Docker setup used to run **xin** against a real JMAP server (Stalwart).

## Ports (localhost only)

- HTTP (JMAP + discovery + management API): `127.0.0.1:39090`
  - `/.well-known/jmap`
  - `/jmap`
  - `/api/*`
- SMTP (fixture injection): `127.0.0.1:32525`

These ports were chosen to avoid common defaults and reduce collisions.

## Quick start

```bash
cd tests/feature/stalwart
./scripts/up.sh
./scripts/seed.sh
./scripts/check.sh
```

## Notes

- State is stored under `tests/feature/stalwart/.state/` (gitignored).
- Seed domain: `example.org` (local-only)
- Credentials are **local-only** and must not be reused.
