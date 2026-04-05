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
        .query_one("SELECT COUNT(*)::bigint FROM users WHERE email = $1", &[&email])
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

    // First registered user becomes admin
    let count_row = client
        .query_one("SELECT COUNT(*)::bigint FROM users", &[])
        .await
        .ok();
    let user_count: i64 = count_row.map(|r| r.get(0)).unwrap_or(0);
    let role = if user_count == 0 || db::is_bootstrap_admin_email(&email) {
        "admin"
    } else {
        "user"
    };

    // Insert user
    if client
        .execute(
            "INSERT INTO users (id, name, email, password_hash, role) VALUES ($1, $2, $3, $4, $5)",
            &[&user_id, &name, &email, &password_hash, &role],
        )
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to create account."
        }));
    }

    // Create session
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
            "SELECT id, name, email, password_hash, role FROM users WHERE email = $1",
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
        client
            .execute(
                "UPDATE users SET role = 'admin' WHERE id = $1",
                &[&user_id],
            )
            .await
            .ok();
        role = "admin".to_string();
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
                    .execute(
                        "UPDATE users SET role = 'admin' WHERE id = $1",
                        &[&user_id],
                    )
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
