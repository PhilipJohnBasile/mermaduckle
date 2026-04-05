use actix_web::{delete, get, post, put, web, HttpResponse};
use rusqlite::params;
use crate::db::{DbPool, add_audit_event};
use crate::models::*;

#[get("/api/agents")]
pub async fn list_agents(pool: web::Data<DbPool>) -> HttpResponse {
    let conn = pool.get().unwrap();
    let mut stmt = conn.prepare("SELECT id, name, description, type, model, prompt, tools, config, runs, success_rate, avg_latency, cost_per_run, tags, created_at, updated_at FROM agents ORDER BY runs DESC").unwrap();

    let agents: Vec<Agent> = stmt.query_map([], |row| {
        let tools_str: String = row.get::<_, String>(6).unwrap_or_else(|_| "[]".into());
        let config_str: String = row.get::<_, String>(7).unwrap_or_else(|_| "{}".into());
        let tags_str: String = row.get::<_, String>(12).unwrap_or_else(|_| "[]".into());
        Ok(Agent {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            agent_type: row.get(3)?,
            model: row.get(4)?,
            prompt: row.get(5)?,
            tools: serde_json::from_str(&tools_str).unwrap_or(serde_json::json!([])),
            config: serde_json::from_str(&config_str).unwrap_or(serde_json::json!({})),
            runs: row.get(8)?,
            success_rate: row.get(9)?,
            avg_latency: row.get(10)?,
            cost_per_run: row.get(11)?,
            tags: serde_json::from_str(&tags_str).unwrap_or(serde_json::json!([])),
            created_at: row.get(13)?,
            updated_at: row.get(14)?,
        })
    }).unwrap().filter_map(|r| r.ok()).collect();

    HttpResponse::Ok().json(agents)
}

#[get("/api/agents/{id}")]
pub async fn get_agent(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let conn = pool.get().unwrap();

    match conn.query_row(
        "SELECT id, name, description, type, model, prompt, tools, config, runs, success_rate, avg_latency, cost_per_run, tags, created_at, updated_at FROM agents WHERE id = ?1",
        params![id],
        |row| {
            let tools_str: String = row.get::<_, String>(6).unwrap_or_else(|_| "[]".into());
            let config_str: String = row.get::<_, String>(7).unwrap_or_else(|_| "{}".into());
            let tags_str: String = row.get::<_, String>(12).unwrap_or_else(|_| "[]".into());
            Ok(Agent {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                agent_type: row.get(3)?,
                model: row.get(4)?,
                prompt: row.get(5)?,
                tools: serde_json::from_str(&tools_str).unwrap_or(serde_json::json!([])),
                config: serde_json::from_str(&config_str).unwrap_or(serde_json::json!({})),
                runs: row.get(8)?,
                success_rate: row.get(9)?,
                avg_latency: row.get(10)?,
                cost_per_run: row.get(11)?,
                tags: serde_json::from_str(&tags_str).unwrap_or(serde_json::json!([])),
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        },
    ) {
        Ok(a) => HttpResponse::Ok().json(a),
        Err(_) => HttpResponse::NotFound().json(serde_json::json!({"error": "Agent not found"})),
    }
}

#[post("/api/agents")]
pub async fn create_agent(pool: web::Data<DbPool>, body: web::Json<CreateAgentRequest>) -> HttpResponse {
    let id = format!("agent_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("x"));
    let now = chrono::Utc::now().to_rfc3339();
    let conn = pool.get().unwrap();

    conn.execute(
        "INSERT INTO agents (id, name, description, type, model, runs, success_rate, avg_latency, cost_per_run, tags, config, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
        params![
            id,
            body.name,
            body.description,
            body.agent_type.as_deref().unwrap_or("llm"),
            body.model.as_deref().unwrap_or("llama3.2"),
            body.runs.unwrap_or(0),
            body.success_rate.unwrap_or(0.0),
            body.avg_latency.unwrap_or(0),
            body.cost_per_run.unwrap_or(0.02),
            body.tags.as_ref().map(|t| t.to_string()).unwrap_or_else(|| "[]".into()),
            body.config.as_ref().map(|c| c.to_string()).unwrap_or_else(|| "{}".into()),
            now, now
        ],
    ).ok();

    add_audit_event(&conn, "agent_created", "low", "usr_1", "System", "system@mermaduckle.io", "agent", &id, &body.name, &serde_json::json!({}));

    HttpResponse::Ok().json(serde_json::json!({"id": id, "name": body.name}))
}

#[put("/api/agents/{id}")]
pub async fn update_agent(pool: web::Data<DbPool>, path: web::Path<String>, body: web::Json<CreateAgentRequest>) -> HttpResponse {
    let id = path.into_inner();
    let now = chrono::Utc::now().to_rfc3339();
    let conn = pool.get().unwrap();

    conn.execute(
        "UPDATE agents SET name=?1, description=?2, type=?3, model=?4, runs=?5, success_rate=?6, avg_latency=?7, cost_per_run=?8, tags=?9, config=?10, updated_at=?11 WHERE id=?12",
        params![
            body.name,
            body.description,
            body.agent_type.as_deref().unwrap_or("llm"),
            body.model.as_deref().unwrap_or("llama3.2"),
            body.runs.unwrap_or(0),
            body.success_rate.unwrap_or(0.0),
            body.avg_latency.unwrap_or(0),
            body.cost_per_run.unwrap_or(0.02),
            body.tags.as_ref().map(|t| t.to_string()).unwrap_or_else(|| "[]".into()),
            body.config.as_ref().map(|c| c.to_string()).unwrap_or_else(|| "{}".into()),
            now, id
        ],
    ).ok();

    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

#[delete("/api/agents/{id}")]
pub async fn delete_agent(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let conn = pool.get().unwrap();
    conn.execute("DELETE FROM agents WHERE id = ?1", params![id]).ok();
    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

#[post("/api/agents/{id}/chat")]
pub async fn chat_with_agent(pool: web::Data<DbPool>, path: web::Path<String>, body: web::Json<ChatRequest>) -> HttpResponse {
    let id = path.into_inner();
    let conn = pool.get().unwrap();

    // 1. Fetch agent config
    let agent_result = conn.query_row(
        "SELECT model, prompt FROM agents WHERE id = ?1",
        params![id],
        |row| {
            let model: Option<String> = row.get(0)?;
            let prompt: Option<String> = row.get(1)?;
            Ok((model.unwrap_or_else(|| "llama3.2".to_string()), prompt.unwrap_or_else(|| "You are a helpful assistant.".to_string())))
        },
    );

    let (model, system_prompt) = match agent_result {
        Ok(vals) => vals,
        Err(_) => return HttpResponse::NotFound().json(serde_json::json!({"error": "Agent not found"})),
    };

    // 2. Interpolate user message into the prompt
    let formatted_prompt = format!("{}\n\nUser: {}\nAgent:", system_prompt, body.message);
    let ollama_url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());

    // 3. Call Ollama from the Engine
    match mermaduckle_engine::call_ollama(&ollama_url, &model, formatted_prompt).await {
        Ok(response) => {
            // Update agent execution stats
            conn.execute("UPDATE agents SET runs = runs + 1 WHERE id = ?1", params![id]).ok();
            HttpResponse::Ok().json(serde_json::json!({
                "response": response
            }))
        },
        Err(e) => {
            // Log it but return 500 error struct to caller cleanly
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Ollama error: {}", e),
                "isMocked": true,
                "response": format!("[Mocked locally] {}", body.message)
            }))
        }
    }
}
