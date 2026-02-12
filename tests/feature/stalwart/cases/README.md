# Stalwart feature cases (Phase B)

This folder contains **declarative feature cases** executed by `xin-feature`.

Run a single case:

```bash
cd ~/Projects/xin
cargo run --bin xin-feature -- --fresh --case tests/feature/stalwart/cases/self_send.yaml
```

Run all cases in a directory:

```bash
cd ~/Projects/xin
cargo run --bin xin-feature -- --fresh --case-dir tests/feature/stalwart/cases --all
```

## Case YAML DSL (minimal)

Top-level fields:

- `id: <string>`: stable case id.
- `it: <string>` (optional): BDD-style human description shown in runner output.
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
- `say` (optional): extra human-readable lines printed by the runner (BDD-style).
- `env` (optional): step-level env overrides/extra vars (merged on top of case `env`).
- `xin.args: ["messages", "search", ...]`: argv list.
- `retry` (optional): retry the whole step until it passes.
  - `attempts` (default 20)
  - `sleepMs` (default 500)
- `expectOk` (default true): require the envelope `ok=true`.
- `expect`: assertions on JSON output using JSON pointers.
  - `label` (optional): a short description shown on failure
  - `path: /data/items/0/subject`
  - `equals: <yaml value>`
  - `contains: <string>`
  - `exists: true|false` (`true` = pointer exists and is not null; `false` = pointer is missing or null)
    - Note: `exists:false` does *not* mean the value is boolean false; use `equals: false` for that.
- `save`: store values into variables.
  - key: var name
  - value: JSON pointer, e.g. `/data/items/0/emailId`
