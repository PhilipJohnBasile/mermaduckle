use crate::db::{DbPool, add_audit_event};
use crate::models::*;
use actix_web::{HttpResponse, delete, get, post, put, web};

#[get("/api/agents")]
pub async fn list_agents(pool: web::Data<DbPool>) -> HttpResponse {
    let client = pool.get().await.unwrap();
    let rows = client
        .query("SELECT id, name, description, type, model, prompt, tools, config, runs, success_rate, avg_latency, cost_per_run, tags, created_at, updated_at FROM agents ORDER BY runs DESC", &[])
        .await
        .unwrap_or_default();

    let agents: Vec<Agent> = rows
        .iter()
        .map(|row| Agent {
            id: row.get(0),
            name: row.get(1),
            description: row.get(2),
            agent_type: row.get(3),
            model: row.get(4),
            prompt: row.get(5),
            tools: row.get::<_, serde_json::Value>(6),
            config: row.get::<_, serde_json::Value>(7),
            runs: row.get(8),
            success_rate: row.get(9),
            avg_latency: row.get(10),
            cost_per_run: row.get(11),
            tags: row.get::<_, serde_json::Value>(12),
            created_at: row.get(13),
            updated_at: row.get(14),
        })
        .collect();

    HttpResponse::Ok().json(agents)
}

#[get("/api/agents/{id}")]
pub async fn get_agent(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let client = pool.get().await.unwrap();

    match client
        .query_opt(
            "SELECT id, name, description, type, model, prompt, tools, config, runs, success_rate, avg_latency, cost_per_run, tags, created_at, updated_at FROM agents WHERE id = $1",
            &[&id],
        )
        .await
    {
        Ok(Some(row)) => {
            let agent = Agent {
                id: row.get(0),
                name: row.get(1),
                description: row.get(2),
                agent_type: row.get(3),
                model: row.get(4),
                prompt: row.get(5),
                tools: row.get::<_, serde_json::Value>(6),
                config: row.get::<_, serde_json::Value>(7),
                runs: row.get(8),
                success_rate: row.get(9),
                avg_latency: row.get(10),
                cost_per_run: row.get(11),
                tags: row.get::<_, serde_json::Value>(12),
                created_at: row.get(13),
                updated_at: row.get(14),
            };
            HttpResponse::Ok().json(agent)
        }
        _ => HttpResponse::NotFound().json(serde_json::json!({"error": "Agent not found"})),
    }
}

#[post("/api/agents")]
pub async fn create_agent(
    pool: web::Data<DbPool>,
    body: web::Json<CreateAgentRequest>,
) -> HttpResponse {
    let id = format!(
        "agent_{}",
        uuid::Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("x")
    );
    let now = chrono::Utc::now().to_rfc3339();
    let client = pool.get().await.unwrap();

    let agent_type = body.agent_type.as_deref().unwrap_or("llm");
    let model = body.model.as_deref().unwrap_or("llama3.2");
    let runs = body.runs.unwrap_or(0);
    let success_rate = body.success_rate.unwrap_or(0.0);
    let avg_latency = body.avg_latency.unwrap_or(0);
    let cost_per_run = body.cost_per_run.unwrap_or(0.02);
    let tags: serde_json::Value = body.tags.clone().unwrap_or(serde_json::json!([]));
    let config: serde_json::Value = body.config.clone().unwrap_or(serde_json::json!({}));

    client.execute(
        "INSERT INTO agents (id, name, description, type, model, runs, success_rate, avg_latency, cost_per_run, tags, config, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
        &[&id, &body.name, &body.description, &agent_type, &model, &runs, &success_rate, &avg_latency, &cost_per_run, &tags, &config, &now, &now],
    ).await.ok();

    add_audit_event(
        &client,
        "agent_created",
        "low",
        "usr_1",
        "System",
        "system@mermaduckle.io",
        "agent",
        &id,
        &body.name,
        &serde_json::json!({}),
    )
    .await;

    HttpResponse::Ok().json(serde_json::json!({"id": id, "name": body.name}))
}

#[put("/api/agents/{id}")]
pub async fn update_agent(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
    body: web::Json<CreateAgentRequest>,
) -> HttpResponse {
    let id = path.into_inner();
    let now = chrono::Utc::now().to_rfc3339();
    let client = pool.get().await.unwrap();

    let agent_type = body.agent_type.as_deref().unwrap_or("llm");
    let model = body.model.as_deref().unwrap_or("llama3.2");
    let runs = body.runs.unwrap_or(0);
    let success_rate = body.success_rate.unwrap_or(0.0);
    let avg_latency = body.avg_latency.unwrap_or(0);
    let cost_per_run = body.cost_per_run.unwrap_or(0.02);
    let tags: serde_json::Value = body.tags.clone().unwrap_or(serde_json::json!([]));
    let config: serde_json::Value = body.config.clone().unwrap_or(serde_json::json!({}));

    client.execute(
        "UPDATE agents SET name=$1, description=$2, type=$3, model=$4, runs=$5, success_rate=$6, avg_latency=$7, cost_per_run=$8, tags=$9, config=$10, updated_at=$11 WHERE id=$12",
        &[&body.name, &body.description, &agent_type, &model, &runs, &success_rate, &avg_latency, &cost_per_run, &tags, &config, &now, &id],
    ).await.ok();

    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

#[delete("/api/agents/{id}")]
pub async fn delete_agent(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let client = pool.get().await.unwrap();
    client
        .execute("DELETE FROM agents WHERE id = $1", &[&id])
        .await
        .ok();
    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

#[post("/api/agents/{id}/chat")]
pub async fn chat_with_agent(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
    body: web::Json<ChatRequest>,
) -> HttpResponse {
    let id = path.into_inner();
    let client = pool.get().await.unwrap();

    let agent_result = client
        .query_opt(
            "SELECT model, prompt FROM agents WHERE id = $1",
            &[&id],
        )
        .await;

    let (model, system_prompt) = match agent_result {
        Ok(Some(row)) => {
            let model: Option<String> = row.get(0);
            let prompt: Option<String> = row.get(1);
            (
                model.unwrap_or_else(|| "llama3.2".to_string()),
                prompt.unwrap_or_else(|| "You are a helpful assistant.".to_string()),
            )
        }
        _ => {
            return HttpResponse::NotFound().json(serde_json::json!({"error": "Agent not found"}));
        }
    };

    let formatted_prompt = format!("{}\n\nUser: {}\nAgent:", system_prompt, body.message);
    let ollama_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());

    match mermaduckle_engine::call_ollama(&ollama_url, &model, formatted_prompt).await {
        Ok(response) => HttpResponse::Ok().json(serde_json::json!({"response": response})),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": format!("AI connection failed: {}", e)})),
    }
}
