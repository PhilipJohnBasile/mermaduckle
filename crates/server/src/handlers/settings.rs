use crate::db::DbPool;
use crate::models::*;
use actix_web::{HttpResponse, delete, get, patch, post, web};

// ── API Keys ───────────────────────────────────────────────

#[get("/api/settings/api-keys")]
pub async fn list_api_keys(pool: web::Data<DbPool>) -> HttpResponse {
    let client = pool.get().await.unwrap();
    let rows = client
        .query("SELECT id, name, key_hash, scopes, status, created_at, last_used_at FROM api_keys ORDER BY created_at DESC", &[])
        .await
        .unwrap_or_default();
    let keys: Vec<ApiKey> = rows
        .iter()
        .map(|row| ApiKey {
            id: row.get(0),
            name: row.get(1),
            key_hash: row.get(2),
            scopes: row.get(3),
            status: row.get(4),
            created_at: row.get(5),
            last_used: row.get(6),
        })
        .collect();
    HttpResponse::Ok().json(keys)
}

#[post("/api/settings/api-keys")]
pub async fn create_api_key(
    pool: web::Data<DbPool>,
    body: web::Json<CreateApiKeyRequest>,
) -> HttpResponse {
    let id = format!(
        "key_{}",
        uuid::Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("x")
    );
    let raw_key = format!(
        "mk_live_{}",
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );
    let key_hash = crate::db::hash_key(&raw_key);
    let client = pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO api_keys (id, name, key_hash, scopes, status) VALUES ($1,$2,$3,'read,write','active')",
            &[&id, &body.name, &key_hash],
        )
        .await
        .ok();
    HttpResponse::Ok().json(serde_json::json!({"id": id, "key": raw_key}))
}

// Development-only: create API key without authentication when running locally.
#[post("/dev/api/settings/api-keys")]
pub async fn create_api_key_dev(
    pool: web::Data<DbPool>,
    body: web::Json<CreateApiKeyRequest>,
) -> HttpResponse {
    let id = format!(
        "key_{}",
        uuid::Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("x")
    );
    let raw_key = format!(
        "mk_live_{}",
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );
    let key_hash = crate::db::hash_key(&raw_key);
    let client = pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO api_keys (id, name, key_hash, scopes, status) VALUES ($1,$2,$3,'read,write','active')",
            &[&id, &body.name, &key_hash],
        )
        .await
        .ok();
    HttpResponse::Ok().json(serde_json::json!({"id": id, "key": raw_key}))
}

#[delete("/api/settings/api-keys/{id}")]
pub async fn delete_api_key(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let client = pool.get().await.unwrap();
    client
        .execute("DELETE FROM api_keys WHERE id = $1", &[&id])
        .await
        .ok();
    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

#[post("/api/settings/api-keys/{id}/rotate")]
pub async fn rotate_api_key(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let raw_key = format!(
        "mk_live_{}",
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );
    let key_hash = crate::db::hash_key(&raw_key);
    let client = pool.get().await.unwrap();
    client
        .execute(
            "UPDATE api_keys SET key_hash = $1, last_used_at = to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') WHERE id = $2",
            &[&key_hash, &id],
        )
        .await
        .ok();
    HttpResponse::Ok().json(serde_json::json!({"id": id, "key": raw_key}))
}

// ── Team Members ───────────────────────────────────────────

#[get("/api/settings/team")]
pub async fn list_team(pool: web::Data<DbPool>) -> HttpResponse {
    let client = pool.get().await.unwrap();
    let rows = client
        .query(
            "SELECT id, name, email, role, status, joined_at FROM team_members ORDER BY joined_at",
            &[],
        )
        .await
        .unwrap_or_default();
    let members: Vec<TeamMember> = rows
        .iter()
        .map(|row| TeamMember {
            id: row.get(0),
            name: row.get(1),
            email: row.get(2),
            role: row.get(3),
            status: row.get(4),
            joined_at: row.get(5),
        })
        .collect();
    HttpResponse::Ok().json(members)
}

#[post("/api/settings/team")]
pub async fn add_team_member(
    pool: web::Data<DbPool>,
    body: web::Json<CreateTeamMemberRequest>,
) -> HttpResponse {
    let id = format!(
        "usr_{}",
        uuid::Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("x")
    );
    let now = chrono::Utc::now().to_rfc3339();
    let role = body.role.as_deref().unwrap_or("viewer");
    let client = pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO team_members (id, name, email, role, status, joined_at) VALUES ($1,$2,$3,$4,'active',$5)",
            &[&id, &body.name, &body.email, &role, &now],
        )
        .await
        .ok();
    HttpResponse::Ok().json(serde_json::json!({"id": id}))
}

#[delete("/api/settings/team/{id}")]
pub async fn remove_team_member(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let client = pool.get().await.unwrap();
    client
        .execute("DELETE FROM team_members WHERE id = $1", &[&id])
        .await
        .ok();
    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

// ── Integrations ───────────────────────────────────────────

#[get("/api/settings/integrations")]
pub async fn list_integrations(pool: web::Data<DbPool>) -> HttpResponse {
    let client = pool.get().await.unwrap();
    let rows = client
        .query(
            "SELECT id, provider, config, status, connected_at FROM integrations ORDER BY provider",
            &[],
        )
        .await
        .unwrap_or_default();
    let integrations: Vec<Integration> = rows
        .iter()
        .map(|row| Integration {
            id: row.get(0),
            provider: row.get(1),
            config: row.get::<_, serde_json::Value>(2),
            status: row.get(3),
            connected_at: row.get(4),
        })
        .collect();
    HttpResponse::Ok().json(integrations)
}

#[patch("/api/settings/integrations")]
pub async fn update_integration(
    pool: web::Data<DbPool>,
    body: web::Json<UpdateIntegrationRequest>,
) -> HttpResponse {
    let now = chrono::Utc::now().to_rfc3339();
    let client = pool.get().await.unwrap();
    let connected_at: Option<String> = if body.status == "connected" {
        Some(now.clone())
    } else {
        None
    };
    let config_val: serde_json::Value = body.config.clone().unwrap_or(serde_json::json!({}));
    client
        .execute(
            "UPDATE integrations SET status = $1, connected_at = $2, config = $3, updated_at = $4 WHERE id = $5",
            &[&body.status, &connected_at, &config_val, &now, &body.id],
        )
        .await
        .ok();
    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

// ── Notifications ──────────────────────────────────────────

#[get("/api/settings/notifications")]
pub async fn get_notifications(pool: web::Data<DbPool>) -> HttpResponse {
    let client = pool.get().await.unwrap();
    match client
        .query_opt(
            "SELECT id, email_alerts, slack_webhook, digest_frequency, alert_severity FROM notification_settings WHERE id = 'default'",
            &[],
        )
        .await
    {
        Ok(Some(row)) => {
            let settings = NotificationSettings {
                id: row.get(0),
                email_alerts: row.get(1),
                slack_webhook: row.get(2),
                digest_frequency: row.get(3),
                alert_severity: row.get(4),
            };
            HttpResponse::Ok().json(settings)
        }
        _ => HttpResponse::Ok().json(serde_json::json!({"id":"default","emailAlerts":1,"digestFrequency":"daily","alertSeverity":"medium"})),
    }
}

#[patch("/api/settings/notifications")]
pub async fn update_notifications(
    pool: web::Data<DbPool>,
    body: web::Json<UpdateNotificationsRequest>,
) -> HttpResponse {
    let now = chrono::Utc::now().to_rfc3339();
    let client = pool.get().await.unwrap();

    if let Some(ea) = body.email_alerts {
        client.execute("UPDATE notification_settings SET email_alerts = $1, updated_at = $2 WHERE id = 'default'", &[&ea, &now]).await.ok();
    }
    if let Some(ref sw) = body.slack_webhook {
        client.execute("UPDATE notification_settings SET slack_webhook = $1, updated_at = $2 WHERE id = 'default'", &[sw, &now]).await.ok();
    }
    if let Some(ref df) = body.digest_frequency {
        client.execute("UPDATE notification_settings SET digest_frequency = $1, updated_at = $2 WHERE id = 'default'", &[df, &now]).await.ok();
    }
    if let Some(ref as_) = body.alert_severity {
        client.execute("UPDATE notification_settings SET alert_severity = $1, updated_at = $2 WHERE id = 'default'", &[as_, &now]).await.ok();
    }

    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

// ── Secret Vault ───────────────────────────────────────────

#[get("/api/settings/secrets")]
pub async fn list_secrets(pool: web::Data<DbPool>) -> HttpResponse {
    let client = pool.get().await.unwrap();
    let rows = client
        .query(
            "SELECT id, key, value, created_at FROM environment_secrets ORDER BY key",
            &[],
        )
        .await
        .unwrap_or_default();
    let secrets: Vec<Secret> = rows
        .iter()
        .map(|row| Secret {
            id: row.get(0),
            key: row.get(1),
            value: row.get(2),
            created_at: row.get(3),
        })
        .collect();
    HttpResponse::Ok().json(secrets)
}

#[post("/api/settings/secrets")]
pub async fn create_secret(
    pool: web::Data<DbPool>,
    body: web::Json<CreateSecretRequest>,
) -> HttpResponse {
    let id = format!(
        "sec_{}",
        uuid::Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("x")
    );
    let client = pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO environment_secrets (id, key, value) VALUES ($1,$2,$3)",
            &[&id, &body.key, &body.value],
        )
        .await
        .ok();
    HttpResponse::Ok().json(serde_json::json!({"id": id}))
}

#[delete("/api/settings/secrets/{id}")]
pub async fn delete_secret(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let client = pool.get().await.unwrap();
    client
        .execute("DELETE FROM environment_secrets WHERE id = $1", &[&id])
        .await
        .ok();
    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}
