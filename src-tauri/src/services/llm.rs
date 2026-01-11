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

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmToolToggle {
    pub id: String,
    pub enabled: bool,
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
) -> Result<String, String> {
    let provider = profile.provider.to_lowercase();
    let base_url = resolve_base_url(profile);
    if base_url.is_empty() {
        return Err("Base URL is required".to_string());
    }
    if provider != "local" && profile.api_key.trim().is_empty() {
        return Err("API key is required".to_string());
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|e| e.to_string())?;

    if provider == "anthropic" {
        return request_anthropic(&client, profile, &base_url, system_prompt, user_prompt).await;
    }

    request_openai_compatible(&client, profile, &base_url, system_prompt, user_prompt).await
}

fn resolve_base_url(profile: &LlmProfile) -> String {
    if !profile.base_url.trim().is_empty() {
        return profile.base_url.trim().trim_end_matches('/').to_string();
    }
    match profile.provider.to_lowercase().as_str() {
        "openai" => "https://api.openai.com/v1".to_string(),
        "anthropic" => "https://api.anthropic.com/v1".to_string(),
        "local" => "http://localhost:11434/v1".to_string(),
        _ => "".to_string(),
    }
}

fn openai_chat_url(base_url: &str) -> String {
    if base_url.contains("/chat/completions") {
        base_url.to_string()
    } else {
        format!("{}/chat/completions", base_url.trim_end_matches('/'))
    }
}

async fn request_openai_compatible(
    client: &Client,
    profile: &LlmProfile,
    base_url: &str,
    system_prompt: &str,
    user_prompt: &str,
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

    let mut request = client.post(url).json(&payload);
    let provider = profile.provider.to_lowercase();
    if provider == "azure" {
        request = request.header("api-key", profile.api_key.trim());
    } else if !profile.api_key.trim().is_empty() {
        request = request.bearer_auth(profile.api_key.trim());
    }

    let response = request.send().await.map_err(|e| e.to_string())?;
    let status = response.status();
    let value: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
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
        .post(url)
        .header("x-api-key", profile.api_key.trim())
        .header("anthropic-version", "2023-06-01")
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    let value: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
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
