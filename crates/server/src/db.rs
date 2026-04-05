use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use deadpool_postgres::{Config, Pool, Runtime};
use rand::rngs::OsRng;

pub type DbPool = Pool;
const DEFAULT_BOOTSTRAP_ADMIN_EMAILS: &[&str] = &["pbasile@basilecom.com"];

pub async fn create_pool(database_url: &str) -> DbPool {
    let mut cfg = Config::new();
    cfg.url = Some(database_url.to_string());

    let tls_connector = native_tls::TlsConnector::builder()
        .build()
        .expect("Failed to build TLS connector");
    let pg_tls = postgres_native_tls::MakeTlsConnector::new(tls_connector);

    let pool = cfg
        .create_pool(Some(Runtime::Tokio1), pg_tls)
        .expect("Failed to create database pool");

    // Initialize schema and seed data
    let client = pool.get().await.expect("Failed to get DB connection");
    init_schema(&client).await;
    seed_data(&client).await;

    pool
}

async fn init_schema(client: &deadpool_postgres::Client) {
    client
        .batch_execute(
            "
        CREATE TABLE IF NOT EXISTS workflows (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            status TEXT DEFAULT 'draft',
            nodes JSONB DEFAULT '[]'::jsonb,
            edges JSONB DEFAULT '[]'::jsonb,
            run_count BIGINT DEFAULT 0,
            last_run_at TEXT,
            schedule TEXT,
            created_at TEXT DEFAULT to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
            updated_at TEXT DEFAULT to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
        );

        CREATE TABLE IF NOT EXISTS environment_secrets (
            id TEXT PRIMARY KEY,
            key TEXT UNIQUE NOT NULL,
            value TEXT NOT NULL,
            created_at TEXT DEFAULT to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
        );

        CREATE TABLE IF NOT EXISTS agents (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            type TEXT DEFAULT 'llm',
            model TEXT,
            prompt TEXT,
            tools JSONB DEFAULT '[]'::jsonb,
            config JSONB DEFAULT '{}'::jsonb,
            runs BIGINT DEFAULT 0,
            success_rate DOUBLE PRECISION DEFAULT 0,
            avg_latency BIGINT DEFAULT 0,
            cost_per_run DOUBLE PRECISION DEFAULT 0,
            tags JSONB DEFAULT '[]'::jsonb,
            created_at TEXT DEFAULT to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
            updated_at TEXT DEFAULT to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
        );

        CREATE TABLE IF NOT EXISTS workflow_runs (
            id TEXT PRIMARY KEY,
            workflow_id TEXT NOT NULL,
            status TEXT DEFAULT 'pending',
            started_at TEXT DEFAULT to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
            completed_at TEXT,
            output TEXT,
            error TEXT,
            logs JSONB DEFAULT '[]'::jsonb,
            context JSONB DEFAULT '{}'::jsonb,
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
            metadata JSONB DEFAULT '{}'::jsonb,
            timestamp TEXT DEFAULT to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
        );

        CREATE TABLE IF NOT EXISTS api_keys (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            key_hash TEXT NOT NULL,
            scopes TEXT DEFAULT 'read,write',
            created_at TEXT DEFAULT to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
            last_used_at TEXT,
            status TEXT DEFAULT 'active'
        );

        CREATE TABLE IF NOT EXISTS team_members (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            role TEXT DEFAULT 'viewer',
            status TEXT DEFAULT 'active',
            joined_at TEXT DEFAULT to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
        );

        CREATE TABLE IF NOT EXISTS notification_settings (
            id TEXT PRIMARY KEY,
            email_alerts BIGINT DEFAULT 1,
            slack_webhook TEXT,
            digest_frequency TEXT DEFAULT 'daily',
            alert_severity TEXT DEFAULT 'medium',
            updated_at TEXT DEFAULT to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
        );

        CREATE TABLE IF NOT EXISTS webhook_logs (
            id TEXT PRIMARY KEY,
            path TEXT NOT NULL,
            method TEXT NOT NULL,
            payload TEXT,
            workflow_id TEXT,
            status TEXT,
            response TEXT,
            created_at TEXT DEFAULT to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
        );

        CREATE TABLE IF NOT EXISTS integrations (
            id TEXT PRIMARY KEY,
            provider TEXT NOT NULL,
            config JSONB DEFAULT '{}'::jsonb,
            status TEXT DEFAULT 'disconnected',
            connected_at TEXT,
            created_at TEXT DEFAULT to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
            updated_at TEXT DEFAULT to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
        );

        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user',
            created_at TEXT DEFAULT to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
        );

        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL REFERENCES users(id),
            created_at TEXT DEFAULT to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
        );

        CREATE TABLE IF NOT EXISTS password_reset_tokens (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL REFERENCES users(id),
            token TEXT NOT NULL UNIQUE,
            expires_at BIGINT NOT NULL,
            created_at BIGINT NOT NULL,
            used_at BIGINT
        );

        CREATE TABLE IF NOT EXISTS migrations (
            id TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL
        );
        ",
        )
        .await
        .expect("Failed to initialize schema");

    // Create indexes idempotently
    client
        .batch_execute(
            "
        CREATE INDEX IF NOT EXISTS idx_password_reset_tokens_user_id ON password_reset_tokens(user_id);
        CREATE INDEX IF NOT EXISTS idx_password_reset_tokens_expires_at ON password_reset_tokens(expires_at);
        ",
        )
        .await
        .ok();

    prune_password_reset_tokens(client).await;
    ensure_bootstrap_admins(client).await;

    // Insert baseline migration marker if not present
    let baseline_id = "baseline_2026_04_04";
    let row = client
        .query_one(
            "SELECT COUNT(*)::bigint FROM migrations WHERE id = $1",
            &[&baseline_id],
        )
        .await
        .ok();
    let exists: i64 = row.map(|r| r.get(0)).unwrap_or(0);
    if exists == 0 {
        let ts = chrono::Utc::now().to_rfc3339();
        client
            .execute(
                "INSERT INTO migrations (id, applied_at) VALUES ($1, $2)",
                &[&baseline_id, &ts],
            )
            .await
            .ok();
    }
}

fn parse_bootstrap_admin_emails(config: Option<&str>) -> Vec<String> {
    let mut emails: Vec<String> = config
        .unwrap_or("")
        .split(',')
        .map(|email| email.trim().to_lowercase())
        .filter(|email| !email.is_empty())
        .collect();

    if emails.is_empty() {
        emails = DEFAULT_BOOTSTRAP_ADMIN_EMAILS
            .iter()
            .map(|email| email.to_string())
            .collect();
    }

    emails.sort();
    emails.dedup();
    emails
}

pub fn configured_bootstrap_admin_emails() -> Vec<String> {
    parse_bootstrap_admin_emails(std::env::var("ADMIN_EMAILS").ok().as_deref())
}

pub fn is_bootstrap_admin_email(email: &str) -> bool {
    let normalized_email = email.trim().to_lowercase();
    configured_bootstrap_admin_emails()
        .iter()
        .any(|candidate| candidate == &normalized_email)
}

pub async fn ensure_bootstrap_admins(client: &deadpool_postgres::Client) {
    let _ = client
        .execute(
            "UPDATE users SET role = 'admin' WHERE id = (SELECT id FROM users ORDER BY created_at ASC LIMIT 1) AND role != 'admin'",
            &[],
        )
        .await;

    for email in configured_bootstrap_admin_emails() {
        let _ = client
            .execute(
                "UPDATE users SET role = 'admin' WHERE lower(email) = $1 AND role != 'admin'",
                &[&email],
            )
            .await;
    }
}

pub async fn prune_password_reset_tokens(client: &deadpool_postgres::Client) {
    let now_ts = chrono::Utc::now().timestamp();
    let _ = client
        .execute(
            "DELETE FROM password_reset_tokens WHERE used_at IS NOT NULL OR expires_at <= $1",
            &[&now_ts],
        )
        .await;
}

pub fn hash_key(key: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(key.as_bytes(), &salt)
        .map(|ph| ph.to_string())
        .unwrap_or_else(|_| key.to_string())
}

async fn seed_data(client: &deadpool_postgres::Client) {
    let now = chrono::Utc::now().to_rfc3339();

    // Check if workflows already exist
    let row = client
        .query_one("SELECT COUNT(*)::bigint FROM workflows", &[])
        .await
        .ok();
    let count: i64 = row.map(|r| r.get(0)).unwrap_or(0);
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
            42801i64,
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

    let n = serde_json::json!([
        {"id":"trigger-1","type":"agentNode","position":{"x":100,"y":200},"data":{"label":"Incoming Webhook","type":"trigger","description":"Data ingested","icon":"Zap","config":{}}},
        {"id":"agent-1","type":"agentNode","position":{"x":350,"y":200},"data":{"label":"Data Enrichment Engine","type":"agent","description":"LLM Processing","icon":"Bot","config":{"model":"llama-3"}}},
        {"id":"action-1","type":"agentNode","position":{"x":600,"y":200},"data":{"label":"API Sync Payload","type":"action","description":"Integration push","icon":"FileText","config":{}}}
    ]);
    let e = serde_json::json!([
        {"id":"e1","source":"trigger-1","target":"agent-1","animated":true},
        {"id":"e2","source":"agent-1","target":"action-1","animated":true}
    ]);

    for (id, name, desc, status, run_count) in &workflows {
        client.execute(
            "INSERT INTO workflows (id, name, description, status, nodes, edges, run_count, last_run_at, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
            &[id, name, desc, status, &n, &e, run_count, &now, &now, &now],
        ).await.ok();
    }

    // ── 2. Agents
    let agents: Vec<(&str, &str, &str, &str, i64, f64, i64, f64, &str)> = vec![
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

    let empty_config = serde_json::json!({});
    for (id, name, desc, model, runs, sr, lat, cost, tags_str) in &agents {
        let tags_val: serde_json::Value =
            serde_json::from_str(tags_str).unwrap_or(serde_json::json!([]));
        client.execute(
            "INSERT INTO agents (id, name, description, type, model, runs, success_rate, avg_latency, cost_per_run, tags, config, created_at, updated_at) VALUES ($1,$2,$3,'llm',$4,$5,$6,$7,$8,$9,$10,$11,$12)",
            &[id, name, desc, model, runs, sr, lat, cost, &tags_val, &empty_config, &now, &now],
        ).await.ok();
    }

    // ── 3. Audit Events
    let users = [
        ("usr_1", "Sarah Chen", "sarah@mermaduckle.io", "admin"),
        ("usr_2", "David Miller", "david@mermaduckle.io", "editor"),
        ("usr_3", "Arjunn Patel", "arjunn@mermaduckle.io", "viewer"),
        ("usr_4", "System Daemon", "system@mermaduckle.io", "system"),
    ];

    for i in 1..=85i64 {
        let u = users[i as usize % users.len()];
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
        let ts = (chrono::Utc::now() - chrono::Duration::hours(i * 3)).to_rfc3339();
        let target_id = format!("wf_{}", w_idx);

        let meta: serde_json::Value = if severity == "high" {
            serde_json::json!({"error":"HTTP 429 Too Many Requests - Rate Limit Exceeded","action":"aborted"})
        } else {
            serde_json::json!({"status":"success","duration_ms":1450})
        };

        client.execute(
            "INSERT INTO audit_events (id, type, severity, actor_id, actor_name, actor_email, target_type, target_id, target_name, metadata, timestamp) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
            &[&id, &evt_type, &severity, &u.0, &u.1, &u.2, &"system", &target_id, &target_name, &meta, &ts],
        ).await.ok();
    }

    // ── 4. Keys & Team & Settings
    let hashed1 = hash_key("sk_live_abc123*********************");
    let hashed2 = hash_key("sk_test_xyz987*********************");
    let hashed3 = hash_key("sk_dev_qwe456**********************");
    client
        .execute(
            "INSERT INTO api_keys (id, name, key_hash, scopes, status) VALUES ($1,$2,$3,$4,$5)",
            &[
                &"key_1",
                &"Production API Sync",
                &hashed1,
                &"read,write,execute",
                &"active",
            ],
        )
        .await
        .ok();
    client
        .execute(
            "INSERT INTO api_keys (id, name, key_hash, scopes, status) VALUES ($1,$2,$3,$4,$5)",
            &[
                &"key_2",
                &"Staging Environment",
                &hashed2,
                &"read,write",
                &"active",
            ],
        )
        .await
        .ok();
    client
        .execute(
            "INSERT INTO api_keys (id, name, key_hash, scopes, status) VALUES ($1,$2,$3,$4,$5)",
            &[
                &"key_3",
                &"Developer Key (Sarah)",
                &hashed3,
                &"read,write,execute",
                &"active",
            ],
        )
        .await
        .ok();

    for (id, name, email, role) in users {
        client.execute(
            "INSERT INTO team_members (id, name, email, role, status, joined_at) VALUES ($1,$2,$3,$4,$5,$6)",
            &[&id, &name, &email, &role, &"active", &now],
        ).await.ok();
    }

    client
        .execute(
            "INSERT INTO integrations (id, provider, config, status) VALUES ($1,$2,$3,$4)",
            &[
                &"intg_1",
                &"OpenAI",
                &serde_json::json!({"apiKey":"sk-..."}),
                &"connected",
            ],
        )
        .await
        .ok();
    client
        .execute(
            "INSERT INTO integrations (id, provider, config, status) VALUES ($1,$2,$3,$4)",
            &[
                &"intg_2",
                &"Slack",
                &serde_json::json!({"workspace":"acme-corp"}),
                &"connected",
            ],
        )
        .await
        .ok();
    client
        .execute(
            "INSERT INTO integrations (id, provider, config, status) VALUES ($1,$2,$3,$4)",
            &[
                &"intg_3",
                &"GitHub",
                &serde_json::json!({"org":"acme-corp"}),
                &"connected",
            ],
        )
        .await
        .ok();
    client
        .execute(
            "INSERT INTO integrations (id, provider, config, status) VALUES ($1,$2,$3,$4)",
            &[
                &"intg_4",
                &"Zendesk",
                &serde_json::json!({"subdomain":"acme.zendesk.com"}),
                &"disconnected",
            ],
        )
        .await
        .ok();

    client.execute(
        "INSERT INTO notification_settings (id, email_alerts, slack_webhook, digest_frequency, alert_severity) VALUES ($1,$2,$3,$4,$5)",
        &[&"default", &1i64, &"https://hooks.slack.com/services/T0123/B456/...", &"daily", &"medium"],
    ).await.ok();
}

// ── Audit Event Helper ─────────────────────────────────────

pub async fn add_audit_event(
    client: &deadpool_postgres::Client,
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
    client.execute(
        "INSERT INTO audit_events (id, type, severity, actor_id, actor_name, actor_email, target_type, target_id, target_name, metadata, timestamp) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
        &[&id, &event_type, &severity, &actor_id, &actor_name, &actor_email, &target_type, &target_id, &target_name, metadata, &ts],
    ).await.ok();
    id
}
