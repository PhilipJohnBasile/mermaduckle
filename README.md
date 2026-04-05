# Mermaduckle — Rust AI Agent Orchestration

Mermaduckle is a self-hosted AI agent orchestration platform implemented as a Rust Cargo workspace. It provides a lightweight Actix-web API server, a small SPA frontend, a workflow execution engine, and tooling for team and integration management.

Quick links
- Server crate: `crates/server`
- Engine: `crates/engine`
- Governance: `crates/governance`
- SDK: `crates/sdk`
- CLI: `crates/cli`

Highlights
- Backend: Rust + Actix-web + Tokio
- DB: SQLite via `rusqlite` + `r2d2` (bundled build for quick local setup)
- Frontend: Vanilla JS SPA served from `crates/server/static`
- API key auth: one-time raw keys returned on creation; keys stored hashed (Argon2)
- Simple migration tracker: `migrations` table with a baseline marker

Quick start (local development)

Prerequisites: Rust toolchain (rustup + cargo), optionally Docker and Ollama if you need the LLM runtime.

Run the server locally (defaults to http://127.0.0.1:3001):

```powershell
# from repo root
cargo run -p mermaduckle-server
```

Routes:
- Marketing site: `/`
- Docs hub: `/docs`
- Control plane SPA: `/app`
- Static assets: `/static`

Local development convenience
- On localhost the SPA may auto-create a temporary development API key (server-side seeded dev admin token) and store it in `localStorage` so you can interact with the UI immediately. This is for local dev only.

Managing API keys
- List keys (protected): GET `/api/settings/api-keys`
- Create key (protected): POST `/api/settings/api-keys` with JSON `{ "name": "My Client" }`. Returns `{ id, key }` where `key` is the raw one-time secret.
- Rotate key (protected): POST `/api/settings/api-keys/{id}/rotate` — returns new raw key.
- Delete key (protected): DELETE `/api/settings/api-keys/{id}`

Note: Protected endpoints require a Bearer token. The server stores key hashes using Argon2 and verifies incoming bearer tokens accordingly.

Health & readiness
- Health endpoint: GET `/api/health` — useful for CI and readiness checks.

Database & migrations
- The server initializes the local SQLite database (`data/app.db` by default), creates required tables, and records a baseline migration in `migrations`. For production use, adopt a real migration tool and consider Postgres.

Environment variables
- `DATABASE_PATH` (default `data/app.db`)
- `OLLAMA_URL` (default `http://localhost:11434`)
- `HOST` (default `0.0.0.0`)
- `PORT` (default `3001`)

Building & testing

```bash
cargo build
cargo test -p mermaduckle-server
```

Development notes
- Seed data includes realistic demo workflows, agents, and a small set of API keys for local testing — these are intended for developer ease only and should not be used in production.

Deployment — Fly.io (recommended)

This repository includes Fly.io deployment artifacts to run `mermaduckle` as a service in the Fly platform. Using Docker on Fly is the simplest way to deploy Rust services with consistent builds and automatic TLS.

Quick steps (full instructions in `docs/deploy/fly.md`):

- Create or select a Fly app (example: `app-rough-dust-5178`).
- Ensure `flyctl` is installed and you're authenticated: `flyctl auth login` (or use an access token).
- The repo contains `deploy/Dockerfile.fly` and a minimal `fly.toml` for builds. To build and deploy:

```bash
flyctl deploy -a <app-name> --dockerfile deploy/Dockerfile.fly
```

- To deploy from the included PowerShell helper:

```powershell
pwsh -File tools/deploy_fly.ps1 -AppName mermaduckle -RemoteOnly
```

- To deploy automatically from GitHub Actions on pushes to `main`, create an app-scoped deploy token and save it as the repository secret `FLY_API_TOKEN`:

```bash
fly tokens create deploy -a <app-name>
```

The workflow file is `.github/workflows/fly-deploy.yml` and runs:

```bash
flyctl deploy --remote-only --config fly.toml
```

- To add your custom domain `mermaduckle.com` to the app:

```bash
flyctl domains add mermaduckle.com -a <app-name>
```

Fly will print exact DNS records to add at your registrar. For an apex/root domain you may need to allocate static IPv4 addresses and create A records:

```bash
flyctl ips allocate-v4 -a <app-name>
# then add A records to your registrar pointing to the returned IP(s)
```

- After DNS is configured and propagated, request TLS certificates via Fly (managed for you):

```bash
flyctl certs create mermaduckle.com -a <app-name>
```

Files in this repo used for Fly deployments:

- `deploy/Dockerfile.fly` — multi-stage Dockerfile that builds a release binary and packages it in a minimal runtime image.
- `fly.toml` — minimal Fly app configuration (app name and service port).
- `docs/deploy/fly.md` — step-by-step Fly deploy and domain instructions.
- `.github/workflows/fly-deploy.yml` — Fly deployment workflow for pushes to `main`.
- `tools/deploy_fly.ps1` — local deployment helper for manual Fly releases.

