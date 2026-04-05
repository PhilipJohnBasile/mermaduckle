# Mermaduckle (Rust Edition) 🦀

Mermaduckle is an enterprise-grade AI Agent Orchestration Platform, now completely rewritten in **100% Rust** for ultimate performance, safety, and a minimal footprint.

## Architecture

The platform has been migrated from a Node.js/Next.js/React monorepo to a high-performance **Cargo Workspace** containing 5 crates:

*   **`mermaduckle-server`**: Actix-web server hosting the JSON API and a blazing-fast Single-Page Application (SPA) driven by Vanilla JS and a native CSS glassmorphism design system.
*   **`mermaduckle-engine`**: The core workflow execution engine, managing context, state transitions, and Ollama HTTP integrations.
*   **`mermaduckle-governance`**: Policy engine enforcing rate limits, cost controls, and content filtering.
*   **`mermaduckle-sdk`**: A native Rust HTTP client wrapping the Mermaduckle REST API.
*   **`mermaduckle-cli`**: A robust command-line interface for managing workflows and agents right from your terminal.

## Key Technologies

*   **Backend**: Rust, Actix-web, Tokio
*   **Database**: SQLite (`rusqlite` + `r2d2`) with bundled SQLite, requiring no external database servers.
*   **Frontend**: Vanilla HTML/JS/CSS served under `static/`.
*   **AI Integration**: Direct HTTP integrations with local [Ollama](https://ollama.ai/) instances.

## Getting Started

### Prerequisites

*   Rust (`1.94.1`+)
*   Ollama (running locally on port `11434`)

### Running Locally

```bash
# Compile and run the server (starts on http://localhost:3000)
cargo run -p mermaduckle-server

# Use the CLI to interact with the platform
cargo run -p mermaduckle-cli -- workflows
cargo run -p mermaduckle-cli -- agents
cargo run -p mermaduckle-cli -- run <workflow_id>
```

## Deployment

### Docker

Build and run with Docker:

```bash
docker build -t mermaduckle .
docker run -p 3000:3000 -v $(pwd)/data:/data mermaduckle
```

### Docker Compose (with Ollama)

For full deployment including Ollama:

```bash
docker-compose up -d
```

This starts both Mermaduckle on port 3000 and Ollama on port 11434.

### Environment Variables

- `DATABASE_PATH`: Path to SQLite database (default: `data/app.db`)
- `OLLAMA_URL`: Ollama server URL (default: `http://localhost:11434`)
- `HOST`: Server host (default: `0.0.0.0`)
- `PORT`: Server port (default: `3000`)

## Development

The entire monorepo builds as a single unit:

```bash
cargo build       # Build all crates
cargo test        # Run unit tests across all libraries
cargo clippy      # Run linting
```

## Database migrations & API keys

This repository includes a minimal migration tracker and secure API key handling.

- A `migrations` table is created on startup by the server; a baseline migration is recorded automatically.
- The `integrations` table is created if missing to avoid seed errors on fresh databases.

API keys are created as one-time raw keys and stored hashed (Argon2) in the database. The server verifies incoming bearer tokens by hashing and comparing.

Create a new API key (one-time raw key returned):

```bash
curl -X POST -H "Content-Type: application/json" -d '{"name":"My Frontend"}' http://localhost:3000/api/settings/api-keys -H "Authorization: Bearer <ADMIN_KEY>"
```

Rotate an API key (returns new one-time raw key):

```bash
curl -X POST http://localhost:3000/api/settings/api-keys/<key_id>/rotate -H "Authorization: Bearer <ADMIN_KEY>"
```

Notes:
- The raw API key is shown only once on creation or rotation; it is stored hashed server-side.
- For production prefer Postgres and a dedicated migration tool; the simple tracker here is intended for fast self-hosted setups.

