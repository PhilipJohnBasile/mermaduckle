use crate::models::{ArchitectRequest, ArchitectResponse};
use actix_web::{HttpResponse, post, web};
use mermaduckle_engine::call_ollama;

#[post("/api/architect/generate")]
pub async fn generate_workflow_draft(body: web::Json<ArchitectRequest>) -> HttpResponse {
    let ollama_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());

    let sys_prompt = "You are the Mermaduckle AI Architect. Your task is to generate a valid JSON workflow graph based on a user description.
    Respond ONLY with a JSON object containing 'nodes' and 'edges' arrays. 
    Node types: 'trigger', 'agent', 'condition', 'approval', 'action'.
    Each node MUST have: 'id', 'type', 'data' { 'label', 'description' }, 'position' { 'x', 'y' }.
    Edges MUST have: 'id', 'source', 'target'.
    
    Example:
    {
      \"nodes\": [
        {\"id\": \"start\", \"type\": \"trigger\", \"data\": {\"label\": \"Start\"}, \"position\": {\"x\": 0, \"y\": 0}},
        {\"id\": \"n1\", \"type\": \"agent\", \"data\": {\"label\": \"AI Agent\"}, \"position\": {\"x\": 200, \"y\": 0}}
      ],
      \"edges\": [
        {\"id\": \"e1\", \"source\": \"start\", \"target\": \"n1\"}
      ]
    }";

    let prompt = format!("{} \n\n User Prompt: {}", sys_prompt, body.prompt);

    match call_ollama(&ollama_url, "llama3.2", prompt).await {
        Ok(response) => {
            // Clean up the response to extract JSON (in case Ollama adds markdown backticks)
            let clean_json = response
                .trim_start_matches("```json")
                .trim_end_matches("```")
                .trim();
            if let Ok(draft) = serde_json::from_str::<ArchitectResponse>(clean_json) {
                HttpResponse::Ok().json(draft)
            } else {
                // Fallback if AI fails to format correctly
                HttpResponse::InternalServerError().json(serde_json::json!({"error": "AI failed to generate valid workflow JSON", "raw": response}))
            }
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": format!("Ollama connection failed: {}", e)})),
    }
}
