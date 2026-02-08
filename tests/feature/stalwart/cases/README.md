# Stalwart feature cases (Phase B)

This folder contains **declarative feature cases** executed by `xin-feature`.

Run a single case:

```bash
cd ~/Projects/xin
cargo run --bin xin-feature -- --fresh --case tests/feature/stalwart/cases/self_send.yaml
```

## Case YAML DSL (minimal)

Top-level fields:

- `id: <string>`: stable case id.
- `requiresFresh: <bool>`: if true, runner forces a fresh docker reset.
- `env: { KEY: VALUE }`: environment variables for every `xin ...` invocation.
- `seed`: Stalwart-only seed plan (domain + accounts + optional SMTP injection).
- `steps`: list of steps.

### Variables

The runner supports `${...}` substitution in `env`, `steps[].xin.args`, `steps[].save`, and assertions.

Built-ins:
- `${caseId}`
- `${runId}`

Saved vars:
- Anything you save via `steps[].save`.

### `seed`

Example:

```yaml
seed:
  domain: example.org
  users:
    - user: alice
      pass: alice-pass
    - user: bob
      pass: bob-pass
  smtpInject:
    - authUser: bob
      authPass: bob-pass
      mailFrom: bob@example.org
      rcptTo: [alice@example.org]
      emlFile: tests/feature/stalwart/fixtures/big.eml
```

Notes:
- Users are created via Stalwart **management API** (`/api/principal`).
- `smtpInject` sends a raw RFC822 `.eml` message via SMTP (STARTTLS + AUTH PLAIN/LOGIN).

### `steps[]`

Each step is one `xin ...` subprocess call.

Fields:
- `name` (optional): display name.
- `xin.args: ["messages", "search", ...]`: argv list.
- `retry` (optional): retry the whole step until it passes.
  - `attempts` (default 20)
  - `sleepMs` (default 500)
- `expectOk` (default true): require the envelope `ok=true`.
- `expect`: assertions on JSON output using JSON pointers.
  - `path: /data/items/0/subject`
  - `equals: <yaml value>`
  - `contains: <string>`
  - `exists: true`
- `save`: store values into variables.
  - key: var name
  - value: JSON pointer, e.g. `/data/items/0/emailId`

