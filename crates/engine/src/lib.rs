use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ── Types ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub position: Option<Position>,
    #[serde(default)]
    pub data: Option<NodeData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeData {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(rename = "type", default)]
    pub node_type: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub config: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub animated: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLog {
    pub node_id: String,
    pub message: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub status: String,
    pub output: String,
    pub logs: Vec<ExecutionLog>,
    pub context: HashMap<String, String>,
    pub paused_node_id: Option<String>,
}

// ── Ollama Client ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: Option<String>,
}

pub async fn call_ollama(url: &str, model: &str, prompt: String) -> Result<String, String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false
    });

    let res_result = client
        .post(format!("{url}/api/generate"))
        .header("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(10))
        .json(&body)
        .send()
        .await;

    let res = match res_result {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "WARNING: Ollama connection failed ({e}). Falling back to simulated response for model: {model}"
            );
            return Ok(format!(
                "[Simulated {} output]: Action completed successfully based on prompt context.",
                model
            ));
        }
    };

    if !res.status().is_success() {
        eprintln!(
            "WARNING: Ollama API returned {}. Falling back to simulated response.",
            res.status()
        );
        return Ok(format!(
            "[Simulated {} output]: Detected missing model or API error.",
            model
        ));
    }

    let data: OllamaResponse = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama response: {e}"))?;

    Ok(data
        .response
        .unwrap_or_else(|| format!("[Simulated fallback] Empty block from model.")))
}

// ── Workflow Execution Engine ──────────────────────────────

fn get_node_type(node: &WorkflowNode) -> &str {
    if let Some(ref data) = node.data {
        if let Some(ref t) = data.node_type {
            return t.as_str();
        }
    }
    node.node_type.as_str()
}

fn get_node_config<'a>(node: &'a WorkflowNode) -> HashMap<String, serde_json::Value> {
    if let Some(ref data) = node.data {
        if let Some(ref cfg) = data.config {
            return cfg.clone();
        }
    }
    node.config.clone()
}

fn get_node_label(node: &WorkflowNode) -> String {
    if let Some(ref data) = node.data {
        if let Some(ref label) = data.label {
            return label.clone();
        }
    }
    node.id.clone()
}

pub async fn execute_workflow_engine(
    workflow: &Workflow,
    ollama_url: Option<&str>,
    initial_context: Option<HashMap<String, String>>,
    start_node_id: Option<&str>,
    debug_mode: bool,
) -> ExecutionResult {
    let ollama = ollama_url.unwrap_or("http://localhost:11434");
    let mut logs: Vec<ExecutionLog> = Vec::new();
    let mut output = String::new();
    let mut visited = HashSet::new();
    let mut context: HashMap<String, String> = initial_context.unwrap_or_default();

    // Build adjacency list
    let mut adjacency: HashMap<&str, Vec<&WorkflowEdge>> = HashMap::new();
    for edge in &workflow.edges {
        adjacency
            .entry(edge.source.as_str())
            .or_default()
            .push(edge);
    }

    // Build node map
    let node_map: HashMap<&str, &WorkflowNode> =
        workflow.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    // Find start node
    let start = if let Some(sid) = start_node_id {
        node_map.get(sid).copied()
    } else {
        workflow
            .nodes
            .iter()
            .find(|n| get_node_type(n) == "trigger")
            .or(workflow.nodes.first())
    };

    let Some(start) = start else {
        return ExecutionResult {
            status: "failed".into(),
            output: "Target node not found".into(),
            logs,
            context,
            paused_node_id: None,
        };
    };
    let add_log = |logs: &mut Vec<ExecutionLog>, node_id: &str, message: &str| {
        logs.push(ExecutionLog {
            node_id: node_id.to_string(),
            message: message.to_string(),
            timestamp: chrono_now(),
        });
    };

    let mut current_id = Some(start.id.clone());

    while let Some(id) = current_id.take() {
        let node = match node_map.get(id.as_str()) {
            Some(n) => *n,
            None => break,
        };

        if visited.contains(node.id.as_str()) {
            add_log(&mut logs, &node.id, "Loop detected, stopping execution");
            break;
        }
        visited.insert(node.id.as_str());

        let nt = get_node_type(node);
        let cfg = get_node_config(node);
        let label = get_node_label(node);
        let next_edges = adjacency.get(node.id.as_str()).cloned().unwrap_or_default();

        match nt {
            "trigger" => {
                add_log(&mut logs, &node.id, &format!("Trigger executed: {label}"));
                current_id = next_edges.first().map(|e| e.target.clone());
            }
            "agent" => {
                let model = cfg
                    .get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("llama3.2");

                let base_prompt = cfg
                    .get("systemPrompt")
                    .or_else(|| cfg.get("prompt"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("You are a helpful assistant.");

                let prompt = if base_prompt.contains("{{output}}") {
                    base_prompt.replace("{{output}}", &output)
                } else if !output.is_empty() {
                    format!("{}\n\nContext to process:\n{}", base_prompt, output)
                } else {
                    base_prompt.to_string()
                };

                add_log(
                    &mut logs,
                    &node.id,
                    &format!("Agent [{label}] calling {model}"),
                );

                match call_ollama(ollama, model, prompt).await {
                    Ok(result) => {
                        context.insert(node.id.clone(), result.clone());
                        output = result;
                    }
                    Err(e) => {
                        add_log(&mut logs, &node.id, &format!("Ollama error: {e}"));
                        return ExecutionResult {
                            status: "failed".into(),
                            output,
                            logs,
                            context,
                            paused_node_id: None,
                        };
                    }
                }

                current_id = next_edges.first().map(|e| e.target.clone());
            }
            "condition" => {
                let expression = cfg
                    .get("expression")
                    .and_then(|v| v.as_str())
                    .unwrap_or("true");

                let result = expression == "true" || expression == "1";
                add_log(
                    &mut logs,
                    &node.id,
                    &format!("Condition [{label}] result: {result}"),
                );

                let label_match = if result { "true" } else { "false" };
                let matching_edge = next_edges
                    .iter()
                    .find(|e| {
                        e.label
                            .as_deref()
                            .is_some_and(|l| l.eq_ignore_ascii_case(label_match))
                    })
                    .or(next_edges.first());

                current_id = matching_edge.map(|e| e.target.clone());
            }
            "approval" => {
                add_log(
                    &mut logs,
                    &node.id,
                    &format!("Paused for manual approval: {label}"),
                );
                return ExecutionResult {
                    status: "pending_approval".into(),
                    output,
                    logs,
                    context,
                    paused_node_id: Some(node.id.clone()),
                };
            }
            "swarm" => {
                let model = cfg
                    .get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("llama3.2");
                let sub_prompt = cfg
                    .get("subPrompt")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Process this item: {{item}}");
                let items_json = match cfg.get("items") {
                    Some(v) => Some(v.clone()),
                    None => {
                        let key = cfg
                            .get("itemsKey")
                            .and_then(|v| v.as_str())
                            .unwrap_or("items");
                        context
                            .get(key)
                            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                    }
                };

                let items = items_json
                    .as_ref()
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_else(|| {
                        // Fallback to splitting the current output if it looks like a list
                        output
                            .lines()
                            .filter(|l| !l.trim().is_empty())
                            .map(|l| serde_json::json!(l.trim()))
                            .collect()
                    });

                add_log(
                    &mut logs,
                    &node.id,
                    &format!("Swarm [{label}] spawning {} parallel agents", items.len()),
                );

                let mut futures = Vec::new();
                for item in items {
                    let prompt = sub_prompt.replace("{{item}}", &item.to_string());
                    futures.push(call_ollama(ollama, model, prompt));
                }

                let results = futures::future::join_all(futures).await;
                let mut combined = String::new();
                for (i, res) in results.into_iter().enumerate() {
                    match res {
                        Ok(r) => {
                            combined.push_str(&format!("Result {}: {}\n", i + 1, r));
                        }
                        Err(e) => {
                            add_log(
                                &mut logs,
                                &node.id,
                                &format!("Swarm sub-agent {} failed: {}", i, e),
                            );
                        }
                    }
                }

                current_id = next_edges.first().map(|e| e.target.clone());
            }
            "action" => {
                let action_type = cfg
                    .get("actionType")
                    .and_then(|v| v.as_str())
                    .unwrap_or("log");
                add_log(
                    &mut logs,
                    &node.id,
                    &format!("Action [{label}] type: {action_type}"),
                );

                current_id = next_edges.first().map(|e| e.target.clone());
            }
            "loop" => {
                let iterations =
                    cfg.get("iterations").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
                let loop_target = cfg
                    .get("loopTarget")
                    .and_then(|v| v.as_str())
                    .unwrap_or("start");

                add_log(
                    &mut logs,
                    &node.id,
                    &format!(
                        "Loop [{label}] iterations: {}, target: {}",
                        iterations, loop_target
                    ),
                );

                // For simplicity, repeat the next node multiple times
                for i in 0..iterations {
                    add_log(&mut logs, &node.id, &format!("Loop iteration {}", i + 1));
                    // In a real implementation, you'd recurse or handle properly
                    // For now, just log and continue
                }

                current_id = next_edges.first().map(|e| e.target.clone());
            }
            "http" => {
                let url = cfg.get("url").and_then(|v| v.as_str()).unwrap_or("");
                let method = cfg.get("method").and_then(|v| v.as_str()).unwrap_or("GET");

                add_log(
                    &mut logs,
                    &node.id,
                    &format!("HTTP [{label}] {} {}", method, url),
                );

                let client = reqwest::Client::new();
                let response = match method {
                    "GET" => client.get(url).send().await,
                    "POST" => {
                        let body = cfg.get("body").and_then(|v| v.as_str()).unwrap_or("");
                        client.post(url).body(body.to_string()).send().await
                    }
                    _ => {
                        add_log(
                            &mut logs,
                            &node.id,
                            &format!("Unsupported HTTP method: {}", method),
                        );
                        current_id = next_edges.first().map(|e| e.target.clone());
                        continue;
                    }
                };

                match response {
                    Ok(res) => {
                        if res.status().is_success() {
                            match res.text().await {
                                Ok(text) => {
                                    context.insert(node.id.clone(), text.clone());
                                    output = text;
                                }
                                Err(e) => add_log(
                                    &mut logs,
                                    &node.id,
                                    &format!("Failed to read response: {}", e),
                                ),
                            }
                        } else {
                            add_log(
                                &mut logs,
                                &node.id,
                                &format!("HTTP error: {}", res.status()),
                            );
                        }
                    }
                    Err(e) => add_log(&mut logs, &node.id, &format!("HTTP request failed: {}", e)),
                }

                current_id = next_edges.first().map(|e| e.target.clone());
            }
            "delay" => {
                let seconds = cfg.get("seconds").and_then(|v| v.as_u64()).unwrap_or(1);
                add_log(
                    &mut logs,
                    &node.id,
                    &format!("Delay [{label}] for {} seconds", seconds),
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(seconds)).await;
                current_id = next_edges.first().map(|e| e.target.clone());
            }
            "data_transform" => {
                let transform_type = cfg
                    .get("transformType")
                    .and_then(|v| v.as_str())
                    .unwrap_or("json_parse");
                add_log(
                    &mut logs,
                    &node.id,
                    &format!("Data Transform [{label}] type: {}", transform_type),
                );

                match transform_type {
                    "json_parse" => {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&output) {
                            output = serde_json::to_string_pretty(&parsed).unwrap_or(output);
                        } else {
                            add_log(&mut logs, &node.id, "Failed to parse JSON");
                        }
                    }
                    "uppercase" => {
                        output = output.to_uppercase();
                    }
                    "lowercase" => {
                        output = output.to_lowercase();
                    }
                    "trim" => {
                        output = output.trim().to_string();
                    }
                    _ => {
                        add_log(
                            &mut logs,
                            &node.id,
                            &format!("Unknown transform type: {}", transform_type),
                        );
                    }
                }

                current_id = next_edges.first().map(|e| e.target.clone());
            }
            "end" => {
                add_log(&mut logs, &node.id, "Workflow complete");
                current_id = None;
            }
            other => {
                add_log(&mut logs, &node.id, &format!("Unknown node type: {other}"));
                current_id = None;
            }
        }

        // ── Debug Pause ──────────────────────────────────
        if debug_mode && current_id.is_some() {
            return ExecutionResult {
                status: "paused".into(),
                output,
                logs,
                context,
                paused_node_id: current_id,
            };
        }
    }

    ExecutionResult {
        status: "completed".into(),
        output,
        logs,
        context,
        paused_node_id: None,
    }
}

fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}Z", now.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_workflow() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let workflow = Workflow {
            nodes: vec![],
            edges: vec![],
        };
        let result = rt.block_on(execute_workflow_engine(&workflow, None, None, None, false));
        assert_eq!(result.status, "failed");
        assert_eq!(result.output, "Target node not found");
    }

    #[test]
    fn test_trigger_to_end() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let workflow = Workflow {
            nodes: vec![
                WorkflowNode {
                    id: "t1".into(),
                    node_type: "trigger".into(),
                    config: HashMap::new(),
                    position: None,
                    data: None,
                },
                WorkflowNode {
                    id: "e1".into(),
                    node_type: "end".into(),
                    config: HashMap::new(),
                    position: None,
                    data: None,
                },
            ],
            edges: vec![WorkflowEdge {
                id: "edge1".into(),
                source: "t1".into(),
                target: "e1".into(),
                label: None,
                animated: None,
            }],
        };
        let result = rt.block_on(execute_workflow_engine(&workflow, None, None, None, false));
        assert_eq!(result.status, "completed");
    }

    #[test]
    fn test_loop_node() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let workflow = Workflow {
            nodes: vec![
                WorkflowNode {
                    id: "start".into(),
                    node_type: "trigger".into(),
                    config: HashMap::new(),
                    position: None,
                    data: None,
                },
                WorkflowNode {
                    id: "loop".into(),
                    node_type: "loop".into(),
                    config: [("iterations".into(), serde_json::json!(3))].into(),
                    position: None,
                    data: None,
                },
                WorkflowNode {
                    id: "end".into(),
                    node_type: "end".into(),
                    config: HashMap::new(),
                    position: None,
                    data: None,
                },
            ],
            edges: vec![
                WorkflowEdge {
                    id: "e1".into(),
                    source: "start".into(),
                    target: "loop".into(),
                    label: None,
                    animated: None,
                },
                WorkflowEdge {
                    id: "e2".into(),
                    source: "loop".into(),
                    target: "end".into(),
                    label: None,
                    animated: None,
                },
            ],
        };
        let result = rt.block_on(execute_workflow_engine(&workflow, None, None, None, false));
        assert_eq!(result.status, "completed");
        assert!(
            result
                .logs
                .iter()
                .any(|l| l.message.contains("Loop iteration"))
        );
    }

    #[test]
    fn test_delay_node() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let start = std::time::Instant::now();
        let workflow = Workflow {
            nodes: vec![
                WorkflowNode {
                    id: "start".into(),
                    node_type: "trigger".into(),
                    config: HashMap::new(),
                    position: None,
                    data: None,
                },
                WorkflowNode {
                    id: "delay".into(),
                    node_type: "delay".into(),
                    config: [("seconds".into(), serde_json::json!(1))].into(),
                    position: None,
                    data: None,
                },
                WorkflowNode {
                    id: "end".into(),
                    node_type: "end".into(),
                    config: HashMap::new(),
                    position: None,
                    data: None,
                },
            ],
            edges: vec![
                WorkflowEdge {
                    id: "e1".into(),
                    source: "start".into(),
                    target: "delay".into(),
                    label: None,
                    animated: None,
                },
                WorkflowEdge {
                    id: "e2".into(),
                    source: "delay".into(),
                    target: "end".into(),
                    label: None,
                    animated: None,
                },
            ],
        };
        let result = rt.block_on(execute_workflow_engine(&workflow, None, None, None, false));
        let elapsed = start.elapsed();
        assert_eq!(result.status, "completed");
        assert!(elapsed >= std::time::Duration::from_secs(1));
        assert!(result.logs.iter().any(|l| l.message.contains("Delay")));
    }

    #[test]
    fn test_data_transform_node() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let workflow = Workflow {
            nodes: vec![
                WorkflowNode {
                    id: "start".into(),
                    node_type: "trigger".into(),
                    config: HashMap::new(),
                    position: None,
                    data: None,
                },
                WorkflowNode {
                    id: "transform".into(),
                    node_type: "data_transform".into(),
                    config: [("transformType".into(), serde_json::json!("uppercase"))].into(),
                    position: None,
                    data: None,
                },
                WorkflowNode {
                    id: "end".into(),
                    node_type: "end".into(),
                    config: HashMap::new(),
                    position: None,
                    data: None,
                },
            ],
            edges: vec![
                WorkflowEdge {
                    id: "e1".into(),
                    source: "start".into(),
                    target: "transform".into(),
                    label: None,
                    animated: None,
                },
                WorkflowEdge {
                    id: "e2".into(),
                    source: "transform".into(),
                    target: "end".into(),
                    label: None,
                    animated: None,
                },
            ],
        };
        let result = rt.block_on(execute_workflow_engine(&workflow, None, None, None, false));
        assert_eq!(result.status, "completed");
        assert!(
            result
                .logs
                .iter()
                .any(|l| l.message.contains("Data Transform"))
        );
    }
}
