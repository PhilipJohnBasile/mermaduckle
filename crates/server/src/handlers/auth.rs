use crate::db::{self, DbPool};
use actix_web::{HttpRequest, HttpResponse, get, post, web};
use argon2::password_hash::{PasswordHash, SaltString};
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use rand::rngs::OsRng;
use rusqlite::params;
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

    let conn = pool.get().unwrap();

    // Check if email already exists
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM users WHERE email = ?1",
            params![email],
            |row| row.get(0),
        )
        .unwrap_or(0);

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

    // First registered user becomes admin, all others are regular users
    let user_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
        .unwrap_or(0);
    let role = if user_count == 0 || db::is_bootstrap_admin_email(&email) {
        "admin"
    } else {
        "user"
    };

    // Insert user
    if let Err(_) = conn.execute(
        "INSERT INTO users (id, name, email, password_hash, role) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![user_id, name, email, password_hash, role],
    ) {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to create account."
        }));
    }

    // Create session
    conn.execute(
        "INSERT INTO sessions (id, user_id) VALUES (?1, ?2)",
        params![session_id, user_id],
    )
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

    conn.execute(
        "INSERT INTO api_keys (id, name, key_hash, scopes, status) VALUES (?1, ?2, ?3, 'read,write,execute', 'active')",
        params![key_id, format!("{}'s key", name), key_hash],
    )
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

    let conn = pool.get().unwrap();

    let user = conn.query_row(
        "SELECT id, name, email, password_hash, role FROM users WHERE email = ?1",
        params![email],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        },
    );

    let (user_id, name, user_email, stored_hash, mut role) = match user {
        Ok(u) => u,
        Err(_) => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Invalid email or password."
            }));
        }
    };

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

    if db::is_bootstrap_admin_email(&user_email) && role != "admin" {
        conn.execute(
            "UPDATE users SET role = 'admin' WHERE id = ?1",
            params![user_id],
        )
        .ok();
        role = "admin".to_string();
    }

    // Create session
    let session_id = format!("ses_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
    conn.execute(
        "INSERT INTO sessions (id, user_id) VALUES (?1, ?2)",
        params![session_id, user_id],
    )
    .ok();

    // Find or create an API key for this user
    let existing_key: Option<String> = conn
        .query_row(
            "SELECT id FROM api_keys WHERE name LIKE ?1 AND status = 'active' LIMIT 1",
            params![format!("{}%", name)],
            |row| row.get(0),
        )
        .ok();

    let raw_key = format!(
        "mk_live_{}",
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );
    let key_hash = db::hash_key(&raw_key);

    if let Some(existing_id) = existing_key {
        // Rotate the existing key so user gets a fresh one
        conn.execute(
            "UPDATE api_keys SET key_hash = ?1, last_used_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![key_hash, existing_id],
        )
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
        conn.execute(
            "INSERT INTO api_keys (id, name, key_hash, scopes, status) VALUES (?1, ?2, ?3, 'read,write,execute', 'active')",
            params![key_id, format!("{}'s key", name), key_hash],
        )
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

    let conn = pool.get().unwrap();
    let user = conn.query_row(
        "SELECT u.id, u.name, u.email, u.role FROM sessions s JOIN users u ON s.user_id = u.id WHERE s.id = ?1",
        params![session_id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        },
    );

    match user {
        Ok((user_id, name, email, mut role)) => {
            if db::is_bootstrap_admin_email(&email) && role != "admin" {
                conn.execute(
                    "UPDATE users SET role = 'admin' WHERE id = ?1",
                    params![user_id],
                )
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
        Err(_) => HttpResponse::Unauthorized().json(serde_json::json!({
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
        let conn = pool.get().unwrap();
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])
            .ok();
    }
    HttpResponse::Ok().json(serde_json::json!({ "success": true }))
}
