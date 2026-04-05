# Mermaduckle — Rust AI Workflow Control Plane

Mermaduckle is an open-source AI workflow control plane implemented as a Rust Cargo workspace. It includes a public demo, a limited hosted beta, a lightweight Actix-web API server, a small SPA frontend, a workflow execution engine, and tooling for approvals, audit, agents, and integrations.

Quick links
- Server crate: `crates/server`
- Engine: `crates/engine`
- Governance: `crates/governance`
- SDK: `crates/sdk`
- CLI: `crates/cli`

Highlights
- Backend: Rust + Actix-web + Tokio
- DB: PostgreSQL via `deadpool-postgres`
- Frontend: Vanilla JS SPA served from `crates/server/static`
- API key auth: one-time raw keys returned on creation; keys stored hashed (Argon2)
- Public demo route plus hosted beta route served from the same app

Quick start (local development)

Prerequisites: Rust toolchain (rustup + cargo), a Postgres database exposed via `DATABASE_URL`, and optionally Docker and Ollama if you need the LLM runtime.

Run the server locally (defaults to http://127.0.0.1:3001):

```powershell
# from repo root
cargo run -p mermaduckle-server
```

Routes:
- Marketing site: `/`
- Public demo: `/demo`
- Docs hub: `/docs`
- Hosted beta SPA: `/app`
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
- The server starts against the Postgres database provided in `DATABASE_URL`. Use Neon or your platform provider when you want a portable Postgres deployment for Fly, Railway, or your own infrastructure.

Environment variables
- `DATABASE_URL` (required)
- `OLLAMA_URL` (optional, default `http://localhost:11434`)
- `OLLAMA_REQUIRED` (optional, default `false`; set to `true` only when Ollama is a required part of the deployment)
- `HOST` (default `0.0.0.0`)
- `PORT` (default `3001`)
- `ADMIN_EMAILS` (optional comma-separated bootstrap admin emails; first registered user is auto-promoted to admin regardless)

Secrets policy
- Use `.env.example` as the template and keep real values in an untracked `.env`.
- Use GitHub Actions secrets for CI, `fly secrets set` for Fly.io, and your platform secret store for hosted environments.
- Do not commit live credentials, database dumps, customer data, or production config to this repository. See `SECURITY.md`.

Building & testing

```bash
cargo build
cargo test -p mermaduckle-server
```

Development notes
- The public demo at `/demo` is intended for sample data only.
- The hosted beta at `/app` is the managed environment for invited users.
- Production deployments should use customer-owned or team-owned infrastructure and Postgres.

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

- If you are retiring the old SQLite-era Fly volume after moving fully to Postgres, run the manual GitHub Actions workflow `.github/workflows/fly-retire-legacy-volume.yml`. It destroys the attached legacy machine and volume, then redeploys with a temporary mount-free Fly config.

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
