use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{create_dir_all, read_to_string, write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmProfile {
    pub profile_name: String,
    pub provider: String,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub provider_configs: HashMap<String, LlmProviderConfig>,
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: u32,
    pub context_window: u32,
    pub stream_responses: bool,
    pub tool_calling: bool,
    pub safety_mode: bool,
    pub retries: u32,
    pub concurrency: u32,
    pub prompt: String,
    pub context_policy: String,
    pub memory_mode: String,
    pub enable_caching: bool,
    pub max_terminal_lines: u32,
    pub redact_secrets: bool,
    pub audit_logs: bool,
    pub tool_toggles: Vec<LlmToolToggle>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum LlmResponseFormat {
    Text,
    ActionJson,
    PlanJson,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmToolToggle {
    pub id: String,
    pub enabled: bool,
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LlmProviderConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmModelFetchRequest {
    pub provider: String,
    pub api_key: String,
    pub base_url: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmModelFetchResponse {
    pub models: Vec<String>,
    pub source_url: String,
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LlmProfileStore {
    pub active: String,
    pub profiles: HashMap<String, LlmProfile>,
}

#[derive(Clone)]
pub struct LlmStore {
    path: Arc<Mutex<PathBuf>>,
    store: Arc<Mutex<LlmProfileStore>>,
}

impl LlmStore {
    pub fn new(root: PathBuf) -> Self {
        let path = root.join(".taurihands").join("llm.json");
        let store = load_store_from_disk(&path);
        Self {
            path: Arc::new(Mutex::new(path)),
            store: Arc::new(Mutex::new(store)),
        }
    }

    pub fn set_root(&self, root: PathBuf) {
        let path = root.join(".taurihands").join("llm.json");
        let store = load_store_from_disk(&path);
        if let Ok(mut current_path) = self.path.lock() {
            *current_path = path;
        }
        if let Ok(mut current_store) = self.store.lock() {
            *current_store = store;
        }
    }

    pub fn get_active_profile(&self) -> Option<LlmProfile> {
        let store = self.store.lock().ok()?.clone();
        if store.active.is_empty() {
            return None;
        }
        store.profiles.get(&store.active).cloned()
    }

    pub fn save_profile(&self, profile: LlmProfile) -> Result<(), String> {
        let mut store = self
            .store
            .lock()
            .map_err(|_| "LLM store lock poisoned".to_string())?;
        let name = if profile.profile_name.trim().is_empty() {
            "Default".to_string()
        } else {
            profile.profile_name.trim().to_string()
        };
        let mut normalized = profile.clone();
        normalized.profile_name = name.clone();
        store.profiles.insert(name.clone(), normalized);
        store.active = name;
        let path = self
            .path
            .lock()
            .map_err(|_| "LLM store path lock poisoned".to_string())?
            .clone();
        save_store_to_disk(&path, &store)
    }

    pub fn snapshot(&self) -> LlmProfileStore {
        self.store
            .lock()
            .map(|store| store.clone())
            .unwrap_or_default()
    }
}

fn load_store_from_disk(path: &PathBuf) -> LlmProfileStore {
    if let Ok(raw) = read_to_string(path) {
        if let Ok(store) = serde_json::from_str::<LlmProfileStore>(&raw) {
            return store;
        }
    }
    LlmProfileStore::default()
}

fn save_store_to_disk(path: &PathBuf, store: &LlmProfileStore) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_vec_pretty(store).map_err(|e| e.to_string())?;
    write(path, data).map_err(|e| e.to_string())
}

pub async fn request_completion(
    profile: &LlmProfile,
    system_prompt: &str,
    user_prompt: &str,
    response_format: LlmResponseFormat,
) -> Result<String, String> {
    let provider = profile.provider.to_lowercase();
    let base_url = resolve_base_url(profile);
    if base_url.is_empty() {
        return Err("Base URL is required".to_string());
    }
    if !matches!(provider.as_str(), "local" | "ollama") && profile.api_key.trim().is_empty() {
        return Err("API key is required".to_string());
    }

    let client = build_http_client()?;

    if provider == "anthropic" {
        return request_anthropic(&client, profile, &base_url, system_prompt, user_prompt).await;
    }
    if provider == "openai" {
        let mode = resolve_openai_request_mode(&base_url);
        if mode == OpenAiRequestMode::Responses {
            return request_openai_responses(
                &client,
                profile,
                &base_url,
                system_prompt,
                user_prompt,
                response_format,
            )
            .await;
        }
    }
    request_openai_compatible(
        &client,
        profile,
        &base_url,
        system_prompt,
        user_prompt,
        response_format,
    )
    .await
}

pub async fn request_completion_stream<F>(
    profile: &LlmProfile,
    system_prompt: &str,
    user_prompt: &str,
    response_format: LlmResponseFormat,
    mut on_chunk: F,
) -> Result<String, String>
where
    F: FnMut(String),
{
    let provider = profile.provider.to_lowercase();
    let base_url = resolve_base_url(profile);
    if base_url.is_empty() {
        return Err("Base URL is required".to_string());
    }
    if !matches!(provider.as_str(), "local" | "ollama") && profile.api_key.trim().is_empty() {
        return Err("API key is required".to_string());
    }

    let client = build_http_client()?;

    if provider == "anthropic" {
        let content = request_anthropic(&client, profile, &base_url, system_prompt, user_prompt).await?;
        on_chunk(content.clone());
        return Ok(content);
    }

    if provider == "openai" {
        let mode = resolve_openai_request_mode(&base_url);
        if profile.stream_responses {
            if mode == OpenAiRequestMode::Responses {
                return request_openai_responses_stream(
                    &client,
                    profile,
                    &base_url,
                    system_prompt,
                    user_prompt,
                    response_format,
                    &mut on_chunk,
                )
                .await;
            }
            return request_openai_compatible_stream(
                &client,
                profile,
                &base_url,
                system_prompt,
                user_prompt,
                response_format,
                &mut on_chunk,
            )
            .await;
        }
        let content = if mode == OpenAiRequestMode::Responses {
            request_openai_responses(
                &client,
                profile,
                &base_url,
                system_prompt,
                user_prompt,
                response_format,
            )
            .await?
        } else {
            request_openai_compatible(
                &client,
                profile,
                &base_url,
                system_prompt,
                user_prompt,
                response_format,
            )
            .await?
        };
        on_chunk(content.clone());
        return Ok(content);
    }

    if profile.stream_responses {
        return request_openai_compatible_stream(
            &client,
            profile,
            &base_url,
            system_prompt,
            user_prompt,
            response_format,
            &mut on_chunk,
        )
        .await;
    }

    let content = request_openai_compatible(
        &client,
        profile,
        &base_url,
        system_prompt,
        user_prompt,
        response_format,
    )
    .await?;
    on_chunk(content.clone());
    Ok(content)
}

pub async fn fetch_models(request: LlmModelFetchRequest) -> Result<LlmModelFetchResponse, String> {
    let provider = request.provider.to_lowercase();
    let client = build_http_client()?;
    match provider.as_str() {
        "openai" => fetch_openai_models(&client, &request).await,
        "local" | "ollama" => fetch_local_models(&client, &provider, &request.base_url).await,
        _ => Err("Model listing is not supported for this provider.".to_string()),
    }
}

fn resolve_base_url(profile: &LlmProfile) -> String {
    let provider = profile.provider.to_lowercase();
    if !profile.base_url.trim().is_empty() {
        let base = profile.base_url.trim().trim_end_matches('/').to_string();
        if matches!(provider.as_str(), "local" | "ollama") {
            return normalize_local_base_url(&base);
        }
        if provider == "openai" {
            return normalize_openai_base_url(&base);
        }
        return base;
    }
    match provider.as_str() {
        "openai" => "https://api.openai.com/v1".to_string(),
        "anthropic" => "https://api.anthropic.com/v1".to_string(),
        "local" => "http://localhost:11434/v1".to_string(),
        "ollama" => "".to_string(),
        _ => "".to_string(),
    }
}

fn normalize_local_base_url(base: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    let lower = trimmed.to_lowercase();
    if lower.contains("/chat/completions") || lower.ends_with("/v1") || lower.contains("/v1/") {
        return trimmed.to_string();
    }
    format!("{}/v1", trimmed)
}

fn normalize_openai_base_url(base: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    let lower = trimmed.to_lowercase();
    if lower.contains("/chat/completions")
        || lower.contains("/responses")
        || lower.ends_with("/v1")
        || lower.contains("/v1/")
    {
        return trimmed.to_string();
    }
    format!("{}/v1", trimmed)
}

#[derive(Clone, Copy, PartialEq)]
enum OpenAiRequestMode {
    Responses,
    ChatCompletions,
}

fn resolve_openai_request_mode(base_url: &str) -> OpenAiRequestMode {
    let lower = base_url.to_lowercase();
    if lower.contains("/chat/completions") {
        OpenAiRequestMode::ChatCompletions
    } else {
        OpenAiRequestMode::Responses
    }
}

fn openai_chat_url(base_url: &str) -> String {
    if base_url.contains("/chat/completions") {
        base_url.to_string()
    } else {
        format!("{}/chat/completions", base_url.trim_end_matches('/'))
    }
}

fn openai_responses_url(base_url: &str) -> String {
    if base_url.contains("/responses") {
        base_url.to_string()
    } else {
        format!("{}/responses", base_url.trim_end_matches('/'))
    }
}

fn openai_json_object_response_format() -> serde_json::Value {
    serde_json::json!({ "type": "json_object" })
}

fn openai_action_schema_response_format() -> serde_json::Value {
    serde_json::json!({
        "type": "json_schema",
        "json_schema": {
            "name": "kernel_action_response",
            "strict": true,
            "schema": {
                "type": "object",
                "properties": {
                    "message": { "type": "string" },
                    "actions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string" },
                                "id": { "type": "string" },
                                "cmd": { "type": "string" },
                                "program": { "type": "string" },
                                "args": { "type": "array", "items": { "type": "string" } },
                                "cwd": { "type": "string" },
                                "path": { "type": "string" },
                                "content": { "type": "string" },
                                "pattern": { "type": "string" },
                                "paths": { "type": "array", "items": { "type": "string" } },
                                "plan": { "type": "object" },
                                "tasks": { "type": "object" },
                                "question": { "type": "string" }
                            },
                            "required": ["type"],
                            "additionalProperties": true
                        }
                    }
                },
                "required": ["actions"],
                "additionalProperties": true
            }
        }
    })
}

fn openai_plan_schema_response_format() -> serde_json::Value {
    serde_json::json!({
        "type": "json_schema",
        "json_schema": {
            "name": "kernel_plan_response",
            "strict": true,
            "schema": {
                "type": "object",
                "properties": {
                    "goal": { "type": "string" },
                    "steps": {
                        "type": "array",
                        "items": {
                            "anyOf": [
                                { "type": "string" },
                                {
                                    "type": "object",
                                    "properties": {
                                        "title": { "type": "string" },
                                        "status": { "type": "string" }
                                    },
                                    "required": ["title"],
                                    "additionalProperties": true
                                }
                            ]
                        },
                        "minItems": 1
                    }
                },
                "required": ["goal", "steps"],
                "additionalProperties": true
            }
        }
    })
}

fn openai_responses_response_format(format: LlmResponseFormat) -> Option<serde_json::Value> {
    match format {
        LlmResponseFormat::ActionJson => Some(openai_action_schema_response_format()),
        LlmResponseFormat::PlanJson => Some(openai_plan_schema_response_format()),
        LlmResponseFormat::Text => None,
    }
}

fn openai_chat_response_format(format: LlmResponseFormat) -> Option<serde_json::Value> {
    match format {
        LlmResponseFormat::Text => None,
        _ => Some(openai_json_object_response_format()),
    }
}

fn openai_models_url(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    let lower = trimmed.to_lowercase();
    if lower.ends_with("/models") {
        return trimmed.to_string();
    }
    let base = strip_openai_endpoint(trimmed);
    let base_trimmed = base.trim_end_matches('/');
    let lower_base = base_trimmed.to_lowercase();
    if lower_base.ends_with("/v1") || lower_base.contains("/v1/") {
        format!("{}/models", base_trimmed)
    } else {
        format!("{}/v1/models", base_trimmed)
    }
}

fn strip_openai_endpoint(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    let lower = trimmed.to_lowercase();
    for marker in ["/responses", "/chat/completions", "/completions"] {
        if let Some(idx) = lower.find(marker) {
            return trimmed[..idx].to_string();
        }
    }
    trimmed.to_string()
}

fn strip_trailing_v1(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    let lower = trimmed.to_lowercase();
    if lower.ends_with("/v1") && trimmed.len() >= 3 {
        return trimmed[..trimmed.len() - 3].to_string();
    }
    trimmed.to_string()
}

fn parse_ollama_tags(value: &serde_json::Value) -> Vec<String> {
    let models = match value.get("models").and_then(|entry| entry.as_array()) {
        Some(models) => models,
        None => return Vec::new(),
    };
    models
        .iter()
        .filter_map(|item| item.get("name").and_then(|name| name.as_str()))
        .map(|name| name.to_string())
        .collect()
}

fn parse_openai_models(value: &serde_json::Value) -> Vec<String> {
    let models = match value.get("data").and_then(|entry| entry.as_array()) {
        Some(models) => models,
        None => return Vec::new(),
    };
    models
        .iter()
        .filter_map(|item| item.get("id").and_then(|name| name.as_str()))
        .map(|name| name.to_string())
        .collect()
}

fn normalize_model_list(models: Vec<String>) -> Vec<String> {
    let mut unique = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for model in models {
        if seen.insert(model.clone()) {
            unique.push(model);
        }
    }
    unique.sort();
    unique
}

fn coerce_json_field(value: &serde_json::Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
        return None;
    }
    if value.is_null() {
        return None;
    }
    let rendered = value.to_string();
    let trimmed = rendered.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn extract_openai_response_text(value: &serde_json::Value) -> Option<String> {
    if let Some(text) = value.get("output_text").and_then(coerce_json_field) {
        return Some(text);
    }
    if let Some(text) = value.get("output_json").and_then(coerce_json_field) {
        return Some(text);
    }
    if let Some(text) = value.get("text").and_then(coerce_json_field) {
        return Some(text);
    }
    let output = value.get("output")?.as_array()?;
    let mut parts = Vec::new();
    for item in output {
        if let Some(text) = item.get("output_text").and_then(coerce_json_field) {
            parts.push(text);
            continue;
        }
        if let Some(text) = item.get("output_json").and_then(coerce_json_field) {
            parts.push(text);
            continue;
        }
        if let Some(text) = item.get("text").and_then(coerce_json_field) {
            parts.push(text);
            continue;
        }
        if let Some(text) = item.get("json").and_then(coerce_json_field) {
            parts.push(text);
            continue;
        }
        if let Some(content) = item.get("content") {
            match content {
                serde_json::Value::String(text) => {
                    if !text.is_empty() {
                        parts.push(text.to_string());
                    }
                }
                serde_json::Value::Array(entries) => {
                    for entry in entries {
                        if let Some(text) = entry.get("output_text").and_then(coerce_json_field) {
                            parts.push(text);
                            continue;
                        }
                        if let Some(text) = entry.get("output_json").and_then(coerce_json_field) {
                            parts.push(text);
                            continue;
                        }
                        if let Some(text) = entry.get("text").and_then(coerce_json_field) {
                            parts.push(text);
                            continue;
                        }
                        if let Some(text) = entry.get("json").and_then(coerce_json_field) {
                            parts.push(text);
                        }
                    }
                }
                serde_json::Value::Object(_) => {
                    if let Some(text) = content.get("output_text").and_then(coerce_json_field) {
                        parts.push(text);
                    } else if let Some(text) =
                        content.get("output_json").and_then(coerce_json_field)
                    {
                        parts.push(text);
                    } else if let Some(text) = content.get("text").and_then(coerce_json_field) {
                        parts.push(text);
                    } else if let Some(text) = content.get("json").and_then(coerce_json_field) {
                        parts.push(text);
                    }
                }
                _ => {}
            }
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(""))
    }
}

fn extract_openai_response_stream_text(
    value: &serde_json::Value,
    allow_full: bool,
) -> Option<String> {
    if let Some(delta) = value.get("delta").and_then(coerce_json_field) {
        if !delta.is_empty() {
            return Some(delta);
        }
    }
    if let Some(text) = value.get("output_text").and_then(coerce_json_field) {
        if !text.is_empty() {
            return Some(text);
        }
    }
    if let Some(text) = value.get("output_json").and_then(coerce_json_field) {
        if !text.is_empty() {
            return Some(text);
        }
    }
    if let Some(text) = value.get("text").and_then(coerce_json_field) {
        if !text.is_empty() {
            return Some(text);
        }
    }
    if let Some(text) = value.get("json").and_then(coerce_json_field) {
        if !text.is_empty() {
            return Some(text);
        }
    }
    if allow_full {
        if let Some(response) = value.get("response") {
            return extract_openai_response_text(response);
        }
    }
    None
}

async fn request_openai_responses_stream<F>(
    client: &Client,
    profile: &LlmProfile,
    base_url: &str,
    system_prompt: &str,
    user_prompt: &str,
    response_format: LlmResponseFormat,
    on_chunk: &mut F,
) -> Result<String, String>
where
    F: FnMut(String),
{
    let url = openai_responses_url(base_url);
    let mut payload = serde_json::json!({
        "model": profile.model,
        "input": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_prompt }
        ],
        "temperature": profile.temperature,
        "top_p": profile.top_p,
        "max_output_tokens": profile.max_tokens,
        "stream": true
    });
    if let Some(format) = openai_responses_response_format(response_format) {
        payload["response_format"] = format;
    }

    let mut request = client.post(&url).json(&payload);
    if !profile.api_key.trim().is_empty() {
        request = request.bearer_auth(profile.api_key.trim());
    }

    let response = request
        .send()
        .await
        .map_err(|e| format_reqwest_error("openai.responses.stream", &url, &e))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) {
            let message = value
                .get("error")
                .and_then(|err| err.get("message"))
                .and_then(|msg| msg.as_str())
                .unwrap_or("LLM request failed");
            return Err(format!("{} (HTTP {})", message, status.as_u16()));
        }
        return Err(format!("LLM request failed (HTTP {})", status.as_u16()));
    }

    let mut full = String::new();
    let mut buffer = String::new();
    let mut stream = response.bytes_stream();
    'outer: while let Some(item) = stream.next().await {
        let chunk = item.map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);
        while let Some(pos) = buffer.find('\n') {
            let mut line = buffer[..pos].to_string();
            buffer = buffer[pos + 1..].to_string();
            line = line.trim_end_matches('\r').to_string();
            if line.is_empty() || !line.starts_with("data:") {
                continue;
            }
            let data = line.trim_start_matches("data:").trim();
            if data == "[DONE]" {
                break 'outer;
            }
            if data.is_empty() {
                continue;
            }
            let value: serde_json::Value = match serde_json::from_str(data) {
                Ok(value) => value,
                Err(_) => continue,
            };
            if let Some(text) = extract_openai_response_stream_text(&value, full.trim().is_empty())
            {
                if !text.is_empty() {
                    full.push_str(&text);
                    on_chunk(text);
                }
            }
        }
    }

    if !buffer.is_empty() {
        for line in buffer.lines() {
            let line = line.trim_end_matches('\r');
            if !line.starts_with("data:") {
                continue;
            }
            let data = line.trim_start_matches("data:").trim();
            if data == "[DONE]" || data.is_empty() {
                continue;
            }
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(text) =
                    extract_openai_response_stream_text(&value, full.trim().is_empty())
                {
                    if !text.is_empty() {
                        full.push_str(&text);
                        on_chunk(text);
                    }
                }
            }
        }
    }

    if full.trim().is_empty() {
        return Err("LLM response is empty".to_string());
    }
    Ok(full)
}

async fn request_openai_responses(
    client: &Client,
    profile: &LlmProfile,
    base_url: &str,
    system_prompt: &str,
    user_prompt: &str,
    response_format: LlmResponseFormat,
) -> Result<String, String> {
    let url = openai_responses_url(base_url);
    let mut payload = serde_json::json!({
        "model": profile.model,
        "input": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_prompt }
        ],
        "temperature": profile.temperature,
        "top_p": profile.top_p,
        "max_output_tokens": profile.max_tokens
    });
    if let Some(format) = openai_responses_response_format(response_format) {
        payload["response_format"] = format;
    }

    let mut request = client.post(&url).json(&payload);
    if !profile.api_key.trim().is_empty() {
        request = request.bearer_auth(profile.api_key.trim());
    }

    let response = request
        .send()
        .await
        .map_err(|e| format_reqwest_error("openai.responses", &url, &e))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format_reqwest_error("openai.responses.read", &url, &e))?;
    let value: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        format!(
            "Invalid JSON response (HTTP {}). error=\"{}\" body_preview=\"{}\"",
            status.as_u16(),
            e,
            truncate_for_error(&body, 800)
        )
    })?;
    if !status.is_success() {
        let message = value
            .get("error")
            .and_then(|err| err.get("message"))
            .and_then(|msg| msg.as_str())
            .unwrap_or("LLM request failed");
        return Err(format!("{} (HTTP {})", message, status.as_u16()));
    }
    let content = extract_openai_response_text(&value)
        .unwrap_or_default()
        .trim()
        .to_string();
    if content.is_empty() {
        return Err("LLM response is empty".to_string());
    }
    Ok(content)
}

async fn fetch_openai_models(
    client: &Client,
    request: &LlmModelFetchRequest,
) -> Result<LlmModelFetchResponse, String> {
    if request.api_key.trim().is_empty() {
        return Err("API key is required.".to_string());
    }
    let base = if request.base_url.trim().is_empty() {
        "https://api.openai.com/v1".to_string()
    } else {
        normalize_openai_base_url(request.base_url.trim())
    };
    let url = openai_models_url(&base);
    let response = client
        .get(&url)
        .bearer_auth(request.api_key.trim())
        .send()
        .await
        .map_err(|e| format_reqwest_error("openai.models", &url, &e))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format_reqwest_error("openai.models.read", &url, &e))?;
    let value: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        format!(
            "Invalid JSON response (HTTP {}). error=\"{}\" body_preview=\"{}\"",
            status.as_u16(),
            e,
            truncate_for_error(&body, 800)
        )
    })?;
    if !status.is_success() {
        let message = value
            .get("error")
            .and_then(|err| err.get("message"))
            .and_then(|msg| msg.as_str())
            .unwrap_or("LLM request failed");
        return Err(format!("{} (HTTP {})", message, status.as_u16()));
    }
    let models = normalize_model_list(parse_openai_models(&value));
    if models.is_empty() {
        return Err("No models found.".to_string());
    }
    Ok(LlmModelFetchResponse {
        models,
        source_url: url,
    })
}

async fn fetch_local_models(
    client: &Client,
    provider: &str,
    base_url: &str,
) -> Result<LlmModelFetchResponse, String> {
    let mut base = base_url.trim().trim_end_matches('/').to_string();
    if base.is_empty() {
        if provider == "local" {
            base = "http://localhost:11434".to_string();
        } else {
            return Err("Base URL is required for Ollama.".to_string());
        }
    }
    let root = strip_trailing_v1(&base);
    let endpoints: Vec<(String, fn(&serde_json::Value) -> Vec<String>)> = vec![
        (format!("{}/api/tags", root), parse_ollama_tags),
        (format!("{}/v1/models", root), parse_openai_models),
    ];

    let mut last_error = String::new();
    for (url, parser) in endpoints {
        let response = match client.get(&url).send().await {
            Ok(response) => response,
            Err(error) => {
                last_error = format_reqwest_error("models.fetch", &url, &error);
                continue;
            }
        };
        let status = response.status();
        if !status.is_success() {
            let reason = status.canonical_reason().unwrap_or("Request failed");
            last_error = format!("HTTP {} {}", status.as_u16(), reason);
            continue;
        }
        let body = match response.text().await {
            Ok(body) => body,
            Err(error) => {
                last_error = format_reqwest_error("models.read", &url, &error);
                continue;
            }
        };
        let value: serde_json::Value = match serde_json::from_str(&body) {
            Ok(value) => value,
            Err(error) => {
                last_error = format!("Invalid JSON: {}", error);
                continue;
            }
        };
        let models = normalize_model_list(parser(&value));
        if !models.is_empty() {
            return Ok(LlmModelFetchResponse {
                models,
                source_url: url,
            });
        }
        last_error = "No models found.".to_string();
    }

    if last_error.is_empty() {
        Err("Unable to load models.".to_string())
    } else {
        Err(last_error)
    }
}

async fn request_openai_compatible_stream<F>(
    client: &Client,
    profile: &LlmProfile,
    base_url: &str,
    system_prompt: &str,
    user_prompt: &str,
    response_format: LlmResponseFormat,
    on_chunk: &mut F,
) -> Result<String, String>
where
    F: FnMut(String),
{
    let url = openai_chat_url(base_url);
    let mut payload = serde_json::json!({
        "model": profile.model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_prompt }
        ],
        "temperature": profile.temperature,
        "top_p": profile.top_p,
        "stream": true
    });
    if use_max_completion_tokens(profile) {
        payload["max_completion_tokens"] = serde_json::json!(profile.max_tokens);
    } else {
        payload["max_tokens"] = serde_json::json!(profile.max_tokens);
    }
    if profile.provider.to_lowercase() == "openai" {
        if let Some(format) = openai_chat_response_format(response_format) {
            payload["response_format"] = format;
        }
    }

    let mut request = client.post(&url).json(&payload);
    let provider = profile.provider.to_lowercase();
    if provider == "azure" {
        request = request.header("api-key", profile.api_key.trim());
    } else if !profile.api_key.trim().is_empty() {
        request = request.bearer_auth(profile.api_key.trim());
    }

    let response = request
        .send()
        .await
        .map_err(|e| format_reqwest_error("openai.stream", &url, &e))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) {
            let message = value
                .get("error")
                .and_then(|err| err.get("message"))
                .and_then(|msg| msg.as_str())
                .unwrap_or("LLM request failed");
            return Err(format!("{} (HTTP {})", message, status.as_u16()));
        }
        return Err(format!("LLM request failed (HTTP {})", status.as_u16()));
    }

    let mut full = String::new();
    let mut buffer = String::new();
    let mut stream = response.bytes_stream();
    'outer: while let Some(item) = stream.next().await {
        let chunk = item.map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);
        while let Some(pos) = buffer.find('\n') {
            let mut line = buffer[..pos].to_string();
            buffer = buffer[pos + 1..].to_string();
            line = line.trim_end_matches('\r').to_string();
            if line.is_empty() || !line.starts_with("data:") {
                continue;
            }
            let data = line.trim_start_matches("data:").trim();
            if data == "[DONE]" {
                break 'outer;
            }
            if data.is_empty() {
                continue;
            }
            let value: serde_json::Value = match serde_json::from_str(data) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let delta = &value["choices"][0]["delta"];
            if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                if !content.is_empty() {
                    full.push_str(content);
                    on_chunk(content.to_string());
                }
                continue;
            }
            if let Some(text) = value["choices"][0]["text"].as_str() {
                if !text.is_empty() {
                    full.push_str(text);
                    on_chunk(text.to_string());
                }
            }
        }
    }

    if !buffer.is_empty() {
        for line in buffer.lines() {
            let line = line.trim_end_matches('\r');
            if !line.starts_with("data:") {
                continue;
            }
            let data = line.trim_start_matches("data:").trim();
            if data == "[DONE]" || data.is_empty() {
                continue;
            }
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(data) {
                let delta = &value["choices"][0]["delta"];
                if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                    if !content.is_empty() {
                        full.push_str(content);
                        on_chunk(content.to_string());
                    }
                    continue;
                }
                if let Some(text) = value["choices"][0]["text"].as_str() {
                    if !text.is_empty() {
                        full.push_str(text);
                        on_chunk(text.to_string());
                    }
                }
            }
        }
    }

    if full.trim().is_empty() {
        return Err("LLM response is empty".to_string());
    }
    Ok(full)
}

async fn request_openai_compatible(
    client: &Client,
    profile: &LlmProfile,
    base_url: &str,
    system_prompt: &str,
    user_prompt: &str,
    response_format: LlmResponseFormat,
) -> Result<String, String> {
    let url = openai_chat_url(base_url);
    let mut payload = serde_json::json!({
        "model": profile.model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_prompt }
        ],
        "temperature": profile.temperature,
        "top_p": profile.top_p
    });
    if use_max_completion_tokens(profile) {
        payload["max_completion_tokens"] = serde_json::json!(profile.max_tokens);
    } else {
        payload["max_tokens"] = serde_json::json!(profile.max_tokens);
    }
    if profile.provider.to_lowercase() == "openai" {
        if let Some(format) = openai_chat_response_format(response_format) {
            payload["response_format"] = format;
        }
    }

    let mut request = client.post(&url).json(&payload);
    let provider = profile.provider.to_lowercase();
    if provider == "azure" {
        request = request.header("api-key", profile.api_key.trim());
    } else if !profile.api_key.trim().is_empty() {
        request = request.bearer_auth(profile.api_key.trim());
    }

    let response = request
        .send()
        .await
        .map_err(|e| format_reqwest_error("openai", &url, &e))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format_reqwest_error("openai.read", &url, &e))?;
    let value: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        format!(
            "Invalid JSON response (HTTP {}). error=\"{}\" body_preview=\"{}\"",
            status.as_u16(),
            e,
            truncate_for_error(&body, 800)
        )
    })?;
    if !status.is_success() {
        let message = value
            .get("error")
            .and_then(|err| err.get("message"))
            .and_then(|msg| msg.as_str())
            .unwrap_or("LLM request failed");
        return Err(format!("{} (HTTP {})", message, status.as_u16()));
    }
    let content = value["choices"][0]["message"]["content"]
        .as_str()
        .or_else(|| value["choices"][0]["text"].as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if content.is_empty() {
        return Err("LLM response is empty".to_string());
    }
    Ok(content)
}

fn use_max_completion_tokens(profile: &LlmProfile) -> bool {
    let provider = profile.provider.to_lowercase();
    if provider != "openai" {
        return false;
    }
    let model = profile.model.to_lowercase();
    model.starts_with("gpt-5") || model.starts_with("o1") || model.starts_with("o3")
}

fn build_http_client() -> Result<Client, String> {
    let builder = Client::builder().timeout(Duration::from_secs(90));
    #[cfg(windows)]
    let builder = builder.use_native_tls();
    #[cfg(not(windows))]
    let builder = builder.use_rustls_tls();
    builder.build().map_err(|e| e.to_string())
}

fn format_reqwest_error(context: &str, url: &str, err: &reqwest::Error) -> String {
    let mut details = Vec::new();
    details.push(format!("Request failed ({})", context));
    details.push(format!("url: {}", url));
    details.push(format!("error: {}", err));
    if err.is_timeout() {
        details.push("reason: timeout".to_string());
    }
    if err.is_connect() {
        details.push("reason: connect".to_string());
    }
    if err.is_request() {
        details.push("reason: request".to_string());
    }
    if err.is_body() {
        details.push("reason: body".to_string());
    }
    if err.is_decode() {
        details.push("reason: decode".to_string());
    }
    if err.is_redirect() {
        details.push("reason: redirect".to_string());
    }
    if err.is_status() {
        details.push("reason: status".to_string());
    }
    if let Some(status) = err.status() {
        details.push(format!("http_status: {}", status.as_u16()));
    }
    if let Some(hint_url) = err.url() {
        details.push(format!("url_hint: {}", hint_url));
    }
    details.join("\n")
}

fn truncate_for_error(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        return value.to_string();
    }
    let mut end = max_len;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &value[..end])
}

async fn request_anthropic(
    client: &Client,
    profile: &LlmProfile,
    base_url: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, String> {
    let url = if base_url.contains("/messages") {
        base_url.to_string()
    } else {
        format!("{}/messages", base_url.trim_end_matches('/'))
    };
    let payload = serde_json::json!({
        "model": profile.model,
        "max_tokens": profile.max_tokens,
        "temperature": profile.temperature,
        "top_p": profile.top_p,
        "system": system_prompt,
        "messages": [
            { "role": "user", "content": user_prompt }
        ]
    });

    let response = client
        .post(url.clone())
        .header("x-api-key", profile.api_key.trim())
        .header("anthropic-version", "2023-06-01")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format_reqwest_error("anthropic", &url, &e))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format_reqwest_error("anthropic.read", &url, &e))?;
    let value: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        format!(
            "Invalid JSON response (HTTP {}). error=\"{}\" body_preview=\"{}\"",
            status.as_u16(),
            e,
            truncate_for_error(&body, 800)
        )
    })?;
    if !status.is_success() {
        let message = value
            .get("error")
            .and_then(|err| err.get("message"))
            .and_then(|msg| msg.as_str())
            .unwrap_or("LLM request failed");
        return Err(format!("{} (HTTP {})", message, status.as_u16()));
    }
    let content = value["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();
    if content.is_empty() {
        return Err("LLM response is empty".to_string());
    }
    Ok(content)
}
