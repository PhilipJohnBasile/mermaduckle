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

Run the server locally (defaults to http://127.0.0.1:3000):

```powershell
# from repo root
cargo run -p mermaduckle-server
```

The SPA is available at `/` and static assets at `/static`.

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
- `PORT` (default `3000`)

Building & testing

```bash
cargo build
cargo test -p mermaduckle-server
```

Development notes
- Seed data includes realistic demo workflows, agents, and a small set of API keys for local testing — these are intended for developer ease only and should not be used in production.

If you want me to, I can also add a short walkthrough showing how to create an API key and paste it into the SPA (or automatically create one for you during local dev). Please tell me which you'd prefer.


