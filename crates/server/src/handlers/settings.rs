use actix_web::{delete, get, patch, post, web, HttpResponse};
use rusqlite::params;
use crate::db::DbPool;
use crate::models::*;

// ── API Keys ───────────────────────────────────────────────

#[get("/api/settings/api-keys")]
pub async fn list_api_keys(pool: web::Data<DbPool>) -> HttpResponse {
    let conn = pool.get().unwrap();
    let mut stmt = conn.prepare("SELECT id, name, key_hash, scopes, status, created_at, last_used_at FROM api_keys ORDER BY created_at DESC").unwrap();
    let keys: Vec<ApiKey> = stmt.query_map([], |row| {
        Ok(ApiKey {
            id: row.get(0)?,
            name: row.get(1)?,
            key_hash: row.get(2)?,
            scopes: row.get(3)?,
            status: row.get(4)?,
            created_at: row.get(5)?,
            last_used: row.get(6)?,
        })
    }).unwrap().filter_map(|r| r.ok()).collect();
    HttpResponse::Ok().json(keys)
}

#[post("/api/settings/api-keys")]
pub async fn create_api_key(pool: web::Data<DbPool>, body: web::Json<CreateApiKeyRequest>) -> HttpResponse {
    let id = format!("key_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("x"));
    let raw_key = format!("mk_live_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
    let key_hash = crate::db::hash_key(&raw_key);
    let conn = pool.get().unwrap();
    conn.execute(
        "INSERT INTO api_keys (id, name, key_hash, scopes, status) VALUES (?1,?2,?3,'read,write','active')",
        params![id, body.name, key_hash],
    ).ok();
    HttpResponse::Ok().json(serde_json::json!({"id": id, "key": raw_key}))
}

#[delete("/api/settings/api-keys/{id}")]
pub async fn delete_api_key(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let conn = pool.get().unwrap();
    conn.execute("DELETE FROM api_keys WHERE id = ?1", params![id]).ok();
    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

#[post("/api/settings/api-keys/{id}/rotate")]
pub async fn rotate_api_key(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let raw_key = format!("mk_live_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
    let key_hash = crate::db::hash_key(&raw_key);
    let conn = pool.get().unwrap();
    conn.execute("UPDATE api_keys SET key_hash = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2", params![key_hash, id]).ok();
    HttpResponse::Ok().json(serde_json::json!({"id": id, "key": raw_key}))
}

// ── Team Members ───────────────────────────────────────────

#[get("/api/settings/team")]
pub async fn list_team(pool: web::Data<DbPool>) -> HttpResponse {
    let conn = pool.get().unwrap();
    let mut stmt = conn.prepare("SELECT id, name, email, role, status, joined_at FROM team_members ORDER BY joined_at").unwrap();
    let members: Vec<TeamMember> = stmt.query_map([], |row| {
        Ok(TeamMember {
            id: row.get(0)?,
            name: row.get(1)?,
            email: row.get(2)?,
            role: row.get(3)?,
            status: row.get(4)?,
            joined_at: row.get(5)?,
        })
    }).unwrap().filter_map(|r| r.ok()).collect();
    HttpResponse::Ok().json(members)
}

#[post("/api/settings/team")]
pub async fn add_team_member(pool: web::Data<DbPool>, body: web::Json<CreateTeamMemberRequest>) -> HttpResponse {
    let id = format!("usr_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("x"));
    let now = chrono::Utc::now().to_rfc3339();
    let role = body.role.as_deref().unwrap_or("viewer");
    let conn = pool.get().unwrap();
    conn.execute(
        "INSERT INTO team_members (id, name, email, role, status, joined_at) VALUES (?1,?2,?3,?4,'active',?5)",
        params![id, body.name, body.email, role, now],
    ).ok();
    HttpResponse::Ok().json(serde_json::json!({"id": id}))
}

#[delete("/api/settings/team/{id}")]
pub async fn remove_team_member(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let conn = pool.get().unwrap();
    conn.execute("DELETE FROM team_members WHERE id = ?1", params![id]).ok();
    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

// ── Integrations ───────────────────────────────────────────

#[get("/api/settings/integrations")]
pub async fn list_integrations(pool: web::Data<DbPool>) -> HttpResponse {
    let conn = pool.get().unwrap();
    let mut stmt = conn.prepare("SELECT id, provider, config, status, connected_at FROM integrations ORDER BY provider").unwrap();
    let integrations: Vec<Integration> = stmt.query_map([], |row| {
        let config_str: String = row.get::<_, String>(2).unwrap_or_else(|_| "{}".into());
        Ok(Integration {
            id: row.get(0)?,
            provider: row.get(1)?,
            config: serde_json::from_str(&config_str).unwrap_or(serde_json::json!({})),
            status: row.get(3)?,
            connected_at: row.get(4)?,
        })
    }).unwrap().filter_map(|r| r.ok()).collect();
    HttpResponse::Ok().json(integrations)
}

#[patch("/api/settings/integrations")]
pub async fn update_integration(pool: web::Data<DbPool>, body: web::Json<UpdateIntegrationRequest>) -> HttpResponse {
    let now = chrono::Utc::now().to_rfc3339();
    let conn = pool.get().unwrap();
    let connected_at = if body.status == "connected" { Some(now.as_str()) } else { None };
    conn.execute(
        "UPDATE integrations SET status = ?1, connected_at = ?2, config = ?3, updated_at = ?4 WHERE id = ?5",
        params![body.status, connected_at, body.config.as_ref().map(|c| c.to_string()).unwrap_or_else(|| "{}".into()), now, body.id],
    ).ok();
    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

// ── Notifications ──────────────────────────────────────────

#[get("/api/settings/notifications")]
pub async fn get_notifications(pool: web::Data<DbPool>) -> HttpResponse {
    let conn = pool.get().unwrap();
    match conn.query_row(
        "SELECT id, email_alerts, slack_webhook, digest_frequency, alert_severity FROM notification_settings WHERE id = 'default'",
        [],
        |row| {
            Ok(NotificationSettings {
                id: row.get(0)?,
                email_alerts: row.get(1)?,
                slack_webhook: row.get(2)?,
                digest_frequency: row.get(3)?,
                alert_severity: row.get(4)?,
            })
        },
    ) {
        Ok(s) => HttpResponse::Ok().json(s),
        Err(_) => HttpResponse::Ok().json(serde_json::json!({"id":"default","emailAlerts":1,"digestFrequency":"daily","alertSeverity":"medium"})),
    }
}

#[patch("/api/settings/notifications")]
pub async fn update_notifications(pool: web::Data<DbPool>, body: web::Json<UpdateNotificationsRequest>) -> HttpResponse {
    let now = chrono::Utc::now().to_rfc3339();
    let conn = pool.get().unwrap();

    if let Some(ea) = body.email_alerts {
        conn.execute("UPDATE notification_settings SET email_alerts = ?1, updated_at = ?2 WHERE id = 'default'", params![ea, now]).ok();
    }
    if let Some(ref sw) = body.slack_webhook {
        conn.execute("UPDATE notification_settings SET slack_webhook = ?1, updated_at = ?2 WHERE id = 'default'", params![sw, now]).ok();
    }
    if let Some(ref df) = body.digest_frequency {
        conn.execute("UPDATE notification_settings SET digest_frequency = ?1, updated_at = ?2 WHERE id = 'default'", params![df, now]).ok();
    }
    if let Some(ref as_) = body.alert_severity {
        conn.execute("UPDATE notification_settings SET alert_severity = ?1, updated_at = ?2 WHERE id = 'default'", params![as_, now]).ok();
    }

    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

// ── Secret Vault ───────────────────────────────────────────

#[get("/api/settings/secrets")]
pub async fn list_secrets(pool: web::Data<DbPool>) -> HttpResponse {
    let conn = pool.get().unwrap();
    let mut stmt = conn.prepare("SELECT id, key, value, created_at FROM environment_secrets ORDER BY key").unwrap();
    let secrets: Vec<Secret> = stmt.query_map([], |row| {
        Ok(Secret {
            id: row.get(0)?,
            key: row.get(1)?,
            value: row.get(2)?,
            created_at: row.get(3)?,
        })
    }).unwrap().filter_map(|r| r.ok()).collect();
    HttpResponse::Ok().json(secrets)
}

#[post("/api/settings/secrets")]
pub async fn create_secret(pool: web::Data<DbPool>, body: web::Json<CreateSecretRequest>) -> HttpResponse {
    let id = format!("sec_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("x"));
    let conn = pool.get().unwrap();
    conn.execute(
        "INSERT INTO environment_secrets (id, key, value) VALUES (?1,?2,?3)",
        params![id, body.key, body.value],
    ).ok();
    HttpResponse::Ok().json(serde_json::json!({"id": id}))
}

#[delete("/api/settings/secrets/{id}")]
pub async fn delete_secret(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let conn = pool.get().unwrap();
    conn.execute("DELETE FROM environment_secrets WHERE id = ?1", params![id]).ok();
    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}
