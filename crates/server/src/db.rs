use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rand::rngs::OsRng;
use rusqlite::params;

pub type DbPool = Pool<SqliteConnectionManager>;

pub fn create_pool(db_path: &str) -> DbPool {
    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(db_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let manager = SqliteConnectionManager::file(db_path);
    let pool = Pool::builder()
        .max_size(8)
        .build(manager)
        .expect("Failed to create database pool");

    // Initialize schema and seed data
    let conn = pool.get().expect("Failed to get DB connection");
    init_schema(&conn);
    seed_data(&conn);

    pool
}

fn init_schema(conn: &rusqlite::Connection) {
    conn.pragma_update(None, "journal_mode", "WAL").ok();

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS workflows (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            status TEXT DEFAULT 'draft',
            nodes TEXT DEFAULT '[]',
            edges TEXT DEFAULT '[]',
            run_count INTEGER DEFAULT 0,
            last_run_at TEXT,
            schedule TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS environment_secrets (
            id TEXT PRIMARY KEY,
            key TEXT UNIQUE NOT NULL,
            value TEXT NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS agents (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            type TEXT DEFAULT 'llm',
            model TEXT,
            prompt TEXT,
            tools TEXT DEFAULT '[]',
            config TEXT DEFAULT '{}',
            runs INTEGER DEFAULT 0,
            success_rate REAL DEFAULT 0,
            avg_latency INTEGER DEFAULT 0,
            cost_per_run REAL DEFAULT 0,
            tags TEXT DEFAULT '[]',
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS workflow_runs (
            id TEXT PRIMARY KEY,
            workflow_id TEXT NOT NULL,
            status TEXT DEFAULT 'pending',
            started_at TEXT DEFAULT CURRENT_TIMESTAMP,
            completed_at TEXT,
            output TEXT,
            error TEXT,
            logs TEXT DEFAULT '[]',
            context TEXT DEFAULT '{}',
            paused_node_id TEXT
        );

        CREATE TABLE IF NOT EXISTS audit_events (
            id TEXT PRIMARY KEY,
            type TEXT NOT NULL,
            severity TEXT DEFAULT 'low',
            actor_id TEXT,
            actor_name TEXT,
            actor_email TEXT,
            target_type TEXT,
            target_id TEXT,
            target_name TEXT,
            metadata TEXT DEFAULT '{}',
            timestamp TEXT DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS api_keys (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            key_hash TEXT NOT NULL,
            scopes TEXT DEFAULT 'read,write',
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            last_used_at TEXT,
            status TEXT DEFAULT 'active'
        );

        CREATE TABLE IF NOT EXISTS team_members (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            role TEXT DEFAULT 'viewer',
            status TEXT DEFAULT 'active',
            joined_at TEXT DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS notification_settings (
            id TEXT PRIMARY KEY DEFAULT 'default',
            email_alerts INTEGER DEFAULT 1,
            slack_webhook TEXT,
            digest_frequency TEXT DEFAULT 'daily',
            alert_severity TEXT DEFAULT 'medium',
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS webhook_logs (
            id TEXT PRIMARY KEY,
            path TEXT NOT NULL,
            method TEXT NOT NULL,
            payload TEXT,
            workflow_id TEXT,
            status TEXT,
            response TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS integrations (
            id TEXT PRIMARY KEY,
            provider TEXT NOT NULL,
            config TEXT DEFAULT '{}',
            status TEXT DEFAULT 'disconnected',
            connected_at TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user',
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id)
        );
        ",
    )
    .ok();

    // Migrations for existing databases (will error if column already exists, which we ignore via .ok())
    let _ = conn.execute("ALTER TABLE workflows ADD COLUMN schedule TEXT", []);
    let _ = conn.execute("ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'user'", []);
    // Promote the earliest registered user to admin (covers existing DBs before role column existed)
    let _ = conn.execute(
        "UPDATE users SET role = 'admin' WHERE id = (SELECT id FROM users ORDER BY created_at ASC LIMIT 1) AND role != 'admin'",
        [],
    );

    // Simple migrations tracking table. Record a baseline migration representing
    // the current schema so future migrations can be applied idempotently.
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS migrations (
            id TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL
        );
        ",
    )
    .ok();

    // Insert a baseline marker if not present
    let baseline_id = "baseline_2026_04_04";
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM migrations WHERE id = ?1",
            params![baseline_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if exists == 0 {
        let ts = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO migrations (id, applied_at) VALUES (?1, ?2)",
            params![baseline_id, ts],
        )
        .ok();
    }
}

pub fn hash_key(key: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(key.as_bytes(), &salt)
        .map(|ph| ph.to_string())
        .unwrap_or_else(|_| key.to_string())
}

fn seed_data(conn: &rusqlite::Connection) {
    let now = chrono::Utc::now().to_rfc3339();

    // Check if workflows already exist
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM workflows", [], |row| row.get(0))
        .unwrap_or(0);
    if count > 0 {
        return;
    }

    println!("Seeding realistic enterprise data...");

    // ── 1. Workflows
    let workflows = vec![
        (
            "wf_1",
            "Automated Triage & Support Resolution",
            "Classifies incoming Zendesk tickets, analyzes sentiment, and drafts responses using specific product documentation.",
            "active",
            42801,
        ),
        (
            "wf_2",
            "Daily SEO Content Generation pipeline",
            "Scrapes trending industry topics, aggregates research, and generates SEO-optimized technical blog posts across 4 languages.",
            "active",
            15234,
        ),
        (
            "wf_3",
            "Code Review & Security Analysis (CI/CD)",
            "Intercepts GitHub PRs to scan for OWASP top 10 vulnerabilities and suggests idiomatic Rust/TS refactors.",
            "active",
            89432,
        ),
        (
            "wf_4",
            "Nightly Database Optimization Agent",
            "Analyzes slow query logs from Postgres and constructs indexing suggestions for the DBA team.",
            "paused",
            1240,
        ),
        (
            "wf_5",
            "Predictive Churn Analysis",
            "Ingests daily Stripe billing failures and usage telemetry to score customer churn probability.",
            "active",
            5600,
        ),
        (
            "wf_6",
            "Sales Lead Enrichment (Apollo + LinkedIn)",
            "Cross-references incoming leads with their social API profiles to generate personalized outreach emails.",
            "active",
            23901,
        ),
        (
            "wf_7",
            "Internal Policy Compliance Checker",
            "Reviews all Slack communications in public channels against HR compliance matrix and triggers warnings.",
            "draft",
            0,
        ),
        (
            "wf_8",
            "Competitor Pricing Scraper",
            "Daily agent workflow that navigates competitor pricing pages, extracts structured data, and updates Salesforce.",
            "active",
            789,
        ),
        (
            "wf_9",
            "Financial Report Summarization (SEC Edgar)",
            "Monitors SEC filings for 50 ticker symbols and summarizes quarterly impact statements.",
            "paused",
            89,
        ),
        (
            "wf_10",
            "Automated UX Feedback Categorization",
            "Parses unstructured App Store reviews into actionable UI/UX Jira tickets with severity scores.",
            "active",
            3410,
        ),
    ];

    for (id, name, desc, status, run_count) in workflows {
        let n: serde_json::Value = serde_json::json!([
            {"id":"trigger-1","type":"agentNode","position":{"x":100,"y":200},"data":{"label":"Incoming Webhook","type":"trigger","description":"Data ingested","icon":"Zap","config":{}}},
            {"id":"agent-1","type":"agentNode","position":{"x":350,"y":200},"data":{"label":"Data Enrichment Engine","type":"agent","description":"LLM Processing","icon":"Bot","config":{"model":"llama-3"}}},
            {"id":"action-1","type":"agentNode","position":{"x":600,"y":200},"data":{"label":"API Sync Payload","type":"action","description":"Integration push","icon":"FileText","config":{}}}
        ]);
        let e: serde_json::Value = serde_json::json!([
            {"id":"e1","source":"trigger-1","target":"agent-1","animated":true},
            {"id":"e2","source":"agent-1","target":"action-1","animated":true}
        ]);

        // Randomize dates to look realistic (past year)
        conn.execute(
            "INSERT INTO workflows (id, name, description, status, nodes, edges, run_count, last_run_at, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![id, name, desc, status, n.to_string(), e.to_string(), run_count, &now, &now, &now],
        ).ok();
    }

    // ── 2. Agents
    let agents = vec![
        (
            "ag_1",
            "Senior Rust Developer",
            "Analyzes codebase context and provides expert idiomatic Rust feedback, identifying memory leaks.",
            "gpt-4-turbo",
            85000,
            99.1,
            4200,
            0.05,
            r#"["code","rust","systems"]"#,
        ),
        (
            "ag_2",
            "Zendesk Support Specialist",
            "Trained on 10,000 support documents. Excels at empathetic, direct troubleshooting steps.",
            "llama3-70b",
            120000,
            94.5,
            1100,
            0.015,
            r#"["support","customer"]"#,
        ),
        (
            "ag_3",
            "SEO Copywriter Pro",
            "Generates marketing copy with optimized keyword density, meta descriptions, and compelling CTR hooks.",
            "claude-3-sonnet",
            45000,
            96.8,
            2400,
            0.03,
            r#"["marketing","copywriting"]"#,
        ),
        (
            "ag_4",
            "Cybersecurity Analyst",
            "Detects SQLi, XSS, SSRF, and logic flaws in code snippets. High accuracy, strict policy rules.",
            "gpt-4",
            15200,
            92.4,
            6700,
            0.06,
            r#"["security","analysis"]"#,
        ),
        (
            "ag_5",
            "Financial Modeler",
            "Extracts key ratios (EBITDA, P/E, PEG) from raw quarterly earnings text and outputs structured JSON.",
            "claude-3-opus",
            3200,
            98.9,
            12000,
            0.15,
            r#"["finance","extraction"]"#,
        ),
        (
            "ag_6",
            "Sales SDR Optimizer",
            "Analyzes lead profile data and drafts highly personalized hyper-targeted cold emails.",
            "gpt-3.5-turbo",
            250000,
            91.2,
            850,
            0.005,
            r#"["sales","outreach"]"#,
        ),
        (
            "ag_7",
            "Data Janitor",
            "Cleans and normalizes messy user-input CSV data, handling edge cases and empty fields.",
            "mistral",
            430000,
            99.8,
            400,
            0.001,
            r#"["data","cleaning"]"#,
        ),
        (
            "ag_8",
            "Legal Contract Summarizer",
            "Reviews legal boilerplates and identifies unusual clauses or extreme liabilities.",
            "gpt-4-turbo",
            1200,
            95.0,
            8900,
            0.10,
            r#"["legal","summary"]"#,
        ),
        (
            "ag_9",
            "HR Compliance Engine",
            "Evaluates internal communications for policy violations, harassment, and confidential leaks.",
            "llama3-8b",
            89000,
            98.1,
            650,
            0.002,
            r#"["hr","compliance"]"#,
        ),
        (
            "ag_10",
            "DevOps Alert Triage",
            "Ingests PagerDuty logs, groups related alerts, and identifies the likely root microservice failure.",
            "gpt-4",
            18500,
            93.3,
            3100,
            0.04,
            r#"["devops","triage"]"#,
        ),
    ];

    for (id, name, desc, model, runs, sr, lat, cost, tags) in agents {
        conn.execute(
            "INSERT INTO agents (id, name, description, type, model, runs, success_rate, avg_latency, cost_per_run, tags, config, created_at, updated_at) VALUES (?1,?2,?3,'llm',?4,?5,?6,?7,?8,?9,'{}',?10,?11)",
            params![id, name, desc, model, runs, sr, lat, cost, tags, &now, &now],
        ).ok();
    }

    // ── 3. Audit Events (mass generation)
    // Generate 100 recent diverse audit logs to populate timelines and graphs
    let users = [
        ("usr_1", "Sarah Chen", "sarah@mermaduckle.io", "admin"),
        ("usr_2", "David Miller", "david@mermaduckle.io", "editor"),
        ("usr_3", "Arjunn Patel", "arjunn@mermaduckle.io", "viewer"),
        ("usr_4", "System Daemon", "system@mermaduckle.io", "system"),
    ];

    for i in 1..=85 {
        let u = users[i % users.len()];
        let w_idx = (i % 10) + 1;
        let severity = if i % 15 == 0 {
            "high"
        } else if i % 5 == 0 {
            "medium"
        } else {
            "low"
        };
        let evt_type = match i % 6 {
            0 => "workflow_run_started",
            1 => "workflow_run_completed",
            2 => "agent_updated",
            3 => "policy_violation",
            4 => "workflow_published",
            _ => "settings_changed",
        };

        let target_name = format!("Workflow wf_{}", w_idx);
        let id = format!("evt_{}", i);

        // Offset timestamps over the past 7 days
        let ts = (chrono::Utc::now() - chrono::Duration::hours(i as i64 * 3)).to_rfc3339();

        let meta = if severity == "high" {
            r#"{"error":"HTTP 429 Too Many Requests - Rate Limit Exceeded","action":"aborted"}"#
        } else {
            r#"{"status":"success","duration_ms":1450}"#
        };

        conn.execute(
            "INSERT INTO audit_events (id, type, severity, actor_id, actor_name, actor_email, target_type, target_id, target_name, metadata, timestamp) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![id, evt_type, severity, u.0, u.1, u.2, "system", format!("wf_{}", w_idx), target_name, meta, ts],
        ).ok();
    }

    // ── 4. Keys & Team & Settings
    let hashed1 = hash_key("sk_live_abc123*********************");
    let hashed2 = hash_key("sk_test_xyz987*********************");
    let hashed3 = hash_key("sk_dev_qwe456**********************");
    conn.execute(
        "INSERT INTO api_keys (id, name, key_hash, scopes, status) VALUES (?1,?2,?3,?4,?5)",
        params![
            "key_1",
            "Production API Sync",
            hashed1,
            "read,write,execute",
            "active"
        ],
    )
    .ok();
    conn.execute(
        "INSERT INTO api_keys (id, name, key_hash, scopes, status) VALUES (?1,?2,?3,?4,?5)",
        params![
            "key_2",
            "Staging Environment",
            hashed2,
            "read,write",
            "active"
        ],
    )
    .ok();
    conn.execute(
        "INSERT INTO api_keys (id, name, key_hash, scopes, status) VALUES (?1,?2,?3,?4,?5)",
        params![
            "key_3",
            "Developer Key (Sarah)",
            hashed3,
            "read,write,execute",
            "active"
        ],
    )
    .ok();

    for (id, name, email, role) in users {
        conn.execute("INSERT INTO team_members (id, name, email, role, status, joined_at) VALUES (?1,?2,?3,?4,?5,?6)", params![id, name, email, role, "active", &now]).ok();
    }

    conn.execute(
        "INSERT INTO integrations (id, provider, config, status) VALUES (?1,?2,?3,?4)",
        params!["intg_1", "OpenAI", r#"{"apiKey":"sk-..."}"#, "connected"],
    )
    .ok();
    conn.execute(
        "INSERT INTO integrations (id, provider, config, status) VALUES (?1,?2,?3,?4)",
        params![
            "intg_2",
            "Slack",
            r#"{"workspace":"acme-corp"}"#,
            "connected"
        ],
    )
    .ok();
    conn.execute(
        "INSERT INTO integrations (id, provider, config, status) VALUES (?1,?2,?3,?4)",
        params!["intg_3", "GitHub", r#"{"org":"acme-corp"}"#, "connected"],
    )
    .ok();
    conn.execute(
        "INSERT INTO integrations (id, provider, config, status) VALUES (?1,?2,?3,?4)",
        params![
            "intg_4",
            "Zendesk",
            r#"{"subdomain":"acme.zendesk.com"}"#,
            "disconnected"
        ],
    )
    .ok();

    conn.execute(
        "INSERT INTO notification_settings (id, email_alerts, slack_webhook, digest_frequency, alert_severity) VALUES (?1,?2,?3,?4,?5)",
        params!["default", 1, "https://hooks.slack.com/services/T0123/B456/...", "daily", "medium"],
    ).ok();
}

// ── Audit Event Helper ─────────────────────────────────────

pub fn add_audit_event(
    conn: &rusqlite::Connection,
    event_type: &str,
    severity: &str,
    actor_id: &str,
    actor_name: &str,
    actor_email: &str,
    target_type: &str,
    target_id: &str,
    target_name: &str,
    metadata: &serde_json::Value,
) -> String {
    let id = format!("evt_{}", chrono::Utc::now().timestamp_millis());
    let ts = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO audit_events (id, type, severity, actor_id, actor_name, actor_email, target_type, target_id, target_name, metadata, timestamp) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![id, event_type, severity, actor_id, actor_name, actor_email, target_type, target_id, target_name, metadata.to_string(), ts],
    ).ok();
    id
}

#[cfg(test)]
mod tests {
    use super::*;
    use argon2::PasswordVerifier;
    use std::path::Path;

    fn temp_db_path() -> String {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "mermaduckle_test_{}.db",
            uuid::Uuid::new_v4().to_string()
        ));
        p.to_string_lossy().to_string()
    }

    #[test]
    fn test_migrations_and_seed() {
        let db_path = temp_db_path();
        if Path::new(&db_path).exists() {
            std::fs::remove_file(&db_path).ok();
        }
        let pool = create_pool(&db_path);
        let conn = pool.get().unwrap();

        // migrations table should have at least the baseline entry
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM migrations", [], |r| r.get(0))
            .unwrap_or(0);
        assert!(count >= 1, "migrations baseline missing");

        // integrations seeded
        let intg_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM integrations", [], |r| r.get(0))
            .unwrap_or(0);
        assert!(intg_count >= 1, "integrations not seeded");

        // api_keys should contain hashed entries (argon2 format)
        let key_hash: String = conn
            .query_row("SELECT key_hash FROM api_keys LIMIT 1", [], |r| r.get(0))
            .unwrap();
        assert!(
            key_hash.contains("$argon2"),
            "api_keys not hashed with argon2"
        );

        std::fs::remove_file(&db_path).ok();
    }

    #[test]
    fn test_hash_key_verify() {
        let raw = "test_secret_key_123";
        let hashed = hash_key(raw);
        let parsed = argon2::password_hash::PasswordHash::new(&hashed).unwrap();
        assert!(
            Argon2::default()
                .verify_password(raw.as_bytes(), &parsed)
                .is_ok()
        );
    }
}
