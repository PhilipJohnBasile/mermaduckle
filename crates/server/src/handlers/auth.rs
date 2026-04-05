use crate::db::{self, DbPool};
use actix_web::{HttpRequest, HttpResponse, get, post, web};
use argon2::password_hash::{PasswordHash, SaltString};
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use rand::rngs::OsRng;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[post("/auth/register")]
pub async fn register(pool: web::Data<DbPool>, body: web::Json<RegisterRequest>) -> HttpResponse {
    let name = body.name.trim();
    let email = body.email.trim().to_lowercase();
    let password = &body.password;

    if name.is_empty() || email.is_empty() || password.len() < 6 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Name, email, and password (min 6 characters) are required."
        }));
    }

    let client = pool.get().await.unwrap();

    // Check if email already exists
    let row = client
        .query_one(
            "SELECT COUNT(*)::bigint FROM users WHERE email = $1",
            &[&email],
        )
        .await
        .ok();
    let exists: i64 = row.map(|r| r.get(0)).unwrap_or(0);

    if exists > 0 {
        return HttpResponse::Conflict().json(serde_json::json!({
            "error": "An account with this email already exists."
        }));
    }

    // Hash password
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = match Argon2::default().hash_password(password.as_bytes(), &salt) {
        Ok(ph) => ph.to_string(),
        Err(_) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to process registration."
            }));
        }
    };

    let user_id = format!(
        "usr_{}",
        uuid::Uuid::new_v4().to_string().replace('-', "")[..12].to_string()
    );
    let session_id = format!("ses_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));

    // First registered user or bootstrap admin is auto-approved; everyone else is pending
    let count_row = client
        .query_one("SELECT COUNT(*)::bigint FROM users", &[])
        .await
        .ok();
    let user_count: i64 = count_row.map(|r| r.get(0)).unwrap_or(0);
    let is_privileged = user_count == 0 || db::is_bootstrap_admin_email(&email);
    let role = if is_privileged { "admin" } else { "user" };
    let status = if is_privileged { "active" } else { "pending" };

    // Insert user
    if client
        .execute(
            "INSERT INTO users (id, name, email, password_hash, role, status) VALUES ($1, $2, $3, $4, $5, $6)",
            &[&user_id, &name, &email, &password_hash, &role, &status],
        )
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to create account."
        }));
    }

    // Pending users don't get a session or API key — they must wait for admin approval
    if status == "pending" {
        return HttpResponse::Ok().json(serde_json::json!({
            "pending": true,
            "message": "Your account has been submitted for approval."
        }));
    }

    // Create session for approved users
    client
        .execute(
            "INSERT INTO sessions (id, user_id) VALUES ($1, $2)",
            &[&session_id, &user_id],
        )
        .await
        .ok();

    // Auto-provision an API key for this user
    let raw_key = format!(
        "mk_live_{}",
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );
    let key_hash = db::hash_key(&raw_key);
    let key_id = format!(
        "key_{}",
        uuid::Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("x")
    );

    client
        .execute(
            "INSERT INTO api_keys (id, name, key_hash, scopes, status) VALUES ($1, $2, $3, 'read,write,execute', 'active')",
            &[&key_id, &format!("{}'s key", name), &key_hash],
        )
        .await
        .ok();

    HttpResponse::Ok().json(serde_json::json!({
        "session": session_id,
        "apiKey": raw_key,
        "user": {
            "id": user_id,
            "name": name,
            "email": email,
            "role": role,
        }
    }))
}

#[post("/auth/login")]
pub async fn login(pool: web::Data<DbPool>, body: web::Json<LoginRequest>) -> HttpResponse {
    let email = body.email.trim().to_lowercase();
    let password = &body.password;

    let client = pool.get().await.unwrap();

    let user = client
        .query_opt(
            "SELECT id, name, email, password_hash, role, COALESCE(status, 'active') FROM users WHERE email = $1",
            &[&email],
        )
        .await;

    let row = match user {
        Ok(Some(r)) => r,
        _ => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Invalid email or password."
            }));
        }
    };

    let user_id: String = row.get(0);
    let name: String = row.get(1);
    let user_email: String = row.get(2);
    let stored_hash: String = row.get(3);
    let mut role: String = row.get(4);
    let mut status: String = row.get(5);

    // Verify password
    let parsed = match PasswordHash::new(&stored_hash) {
        Ok(p) => p,
        Err(_) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Account error. Please contact support."
            }));
        }
    };

    if Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_err()
    {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Invalid email or password."
        }));
    }

    // Bootstrap admins are always promoted and activated
    if db::is_bootstrap_admin_email(&user_email) {
        if role != "admin" || status != "active" {
            client
                .execute(
                    "UPDATE users SET role = 'admin', status = 'active' WHERE id = $1",
                    &[&user_id],
                )
                .await
                .ok();
            role = "admin".to_string();
            status = "active".to_string();
        }
    }

    // Block users who haven't been approved yet
    if status != "active" {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Your account is pending approval. An administrator will review your registration."
        }));
    }

    // Create session
    let session_id = format!("ses_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
    client
        .execute(
            "INSERT INTO sessions (id, user_id) VALUES ($1, $2)",
            &[&session_id, &user_id],
        )
        .await
        .ok();

    // Find or create an API key
    let existing_key = client
        .query_opt(
            "SELECT id FROM api_keys WHERE name LIKE $1 AND status = 'active' LIMIT 1",
            &[&format!("{}%", name)],
        )
        .await
        .ok()
        .flatten();

    let raw_key = format!(
        "mk_live_{}",
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );
    let key_hash = db::hash_key(&raw_key);

    if let Some(existing_row) = existing_key {
        let existing_id: String = existing_row.get(0);
        client
            .execute(
                "UPDATE api_keys SET key_hash = $1, last_used_at = to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') WHERE id = $2",
                &[&key_hash, &existing_id],
            )
            .await
            .ok();
    } else {
        let key_id = format!(
            "key_{}",
            uuid::Uuid::new_v4()
                .to_string()
                .split('-')
                .next()
                .unwrap_or("x")
        );
        client
            .execute(
                "INSERT INTO api_keys (id, name, key_hash, scopes, status) VALUES ($1, $2, $3, 'read,write,execute', 'active')",
                &[&key_id, &format!("{}'s key", name), &key_hash],
            )
            .await
            .ok();
    }

    HttpResponse::Ok().json(serde_json::json!({
        "session": session_id,
        "apiKey": raw_key,
        "user": {
            "id": user_id,
            "name": name,
            "email": user_email,
            "role": role,
        }
    }))
}

#[get("/auth/me")]
pub async fn me(req: HttpRequest, pool: web::Data<DbPool>) -> HttpResponse {
    let session_id = match req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        Some(s) => s.to_string(),
        None => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "No session token."
            }));
        }
    };

    let client = pool.get().await.unwrap();
    let user = client
        .query_opt(
            "SELECT u.id, u.name, u.email, u.role FROM sessions s JOIN users u ON s.user_id = u.id WHERE s.id = $1",
            &[&session_id],
        )
        .await;

    match user {
        Ok(Some(row)) => {
            let user_id: String = row.get(0);
            let name: String = row.get(1);
            let email: String = row.get(2);
            let mut role: String = row.get(3);

            if db::is_bootstrap_admin_email(&email) && role != "admin" {
                client
                    .execute("UPDATE users SET role = 'admin' WHERE id = $1", &[&user_id])
                    .await
                    .ok();
                role = "admin".to_string();
            }

            HttpResponse::Ok().json(serde_json::json!({
                "id": user_id,
                "name": name,
                "email": email,
                "role": role,
            }))
        }
        _ => HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Invalid or expired session."
        })),
    }
}

#[post("/auth/logout")]
pub async fn logout(req: HttpRequest, pool: web::Data<DbPool>) -> HttpResponse {
    if let Some(session_id) = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        let client = pool.get().await.unwrap();
        client
            .execute("DELETE FROM sessions WHERE id = $1", &[&session_id])
            .await
            .ok();
    }
    HttpResponse::Ok().json(serde_json::json!({ "success": true }))
}

// ── Admin: user management ─────────────────────────────────

async fn require_admin(req: &HttpRequest, pool: &web::Data<DbPool>) -> Result<(), HttpResponse> {
    let session_id = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| {
            HttpResponse::Unauthorized().json(serde_json::json!({"error": "No session token."}))
        })?;

    let client = pool.get().await.map_err(|_| {
        HttpResponse::InternalServerError().json(serde_json::json!({"error": "DB error"}))
    })?;

    let row = client
        .query_opt(
            "SELECT u.role FROM sessions s JOIN users u ON s.user_id = u.id WHERE s.id = $1",
            &[&session_id.to_string()],
        )
        .await
        .map_err(|_| {
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "DB error"}))
        })?
        .ok_or_else(|| {
            HttpResponse::Unauthorized().json(serde_json::json!({"error": "Invalid session."}))
        })?;

    let role: String = row.get(0);
    if role != "admin" {
        return Err(
            HttpResponse::Forbidden().json(serde_json::json!({"error": "Admin access required."}))
        );
    }
    Ok(())
}

#[get("/auth/admin/users")]
pub async fn list_all_users(req: HttpRequest, pool: web::Data<DbPool>) -> HttpResponse {
    if let Err(resp) = require_admin(&req, &pool).await {
        return resp;
    }
    let client = pool.get().await.unwrap();
    let rows = client
        .query(
            "SELECT id, name, email, role, COALESCE(status, 'active'), created_at FROM users ORDER BY created_at DESC",
            &[],
        )
        .await
        .unwrap_or_default();

    let users: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.get::<_, String>(0),
                "name": row.get::<_, String>(1),
                "email": row.get::<_, String>(2),
                "role": row.get::<_, String>(3),
                "status": row.get::<_, String>(4),
                "createdAt": row.get::<_, Option<String>>(5),
            })
        })
        .collect();

    HttpResponse::Ok().json(users)
}

#[post("/auth/admin/users/{id}/approve")]
pub async fn approve_user(
    req: HttpRequest,
    pool: web::Data<DbPool>,
    path: web::Path<String>,
) -> HttpResponse {
    if let Err(resp) = require_admin(&req, &pool).await {
        return resp;
    }
    let user_id = path.into_inner();
    let client = pool.get().await.unwrap();
    client
        .execute(
            "UPDATE users SET status = 'active' WHERE id = $1",
            &[&user_id],
        )
        .await
        .ok();
    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

#[post("/auth/admin/users/{id}/reject")]
pub async fn reject_user(
    req: HttpRequest,
    pool: web::Data<DbPool>,
    path: web::Path<String>,
) -> HttpResponse {
    if let Err(resp) = require_admin(&req, &pool).await {
        return resp;
    }
    let user_id = path.into_inner();
    let client = pool.get().await.unwrap();
    // Delete associated sessions and the user record
    client
        .execute("DELETE FROM sessions WHERE user_id = $1", &[&user_id])
        .await
        .ok();
    client
        .execute("DELETE FROM users WHERE id = $1", &[&user_id])
        .await
        .ok();
    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}
