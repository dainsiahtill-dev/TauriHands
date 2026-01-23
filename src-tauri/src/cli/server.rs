use std::path::PathBuf;
use anyhow::Result;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_hdr_async, tungstenite::Message};
use tokio_tungstenite::{tungstenite::protocol::WebSocketConfig, WebSocketStream};
use futures_util::{SinkExt, StreamExt};
use serde_json;

pub async fn start_web_server(workspace: &PathBuf, host: &str, port: u16, open_browser: bool) -> Result<()> {
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr).await?;
    
    println!("🌐 Web server started at http://{}:{}", host, port);
    
    if open_browser {
        if let Err(e) = webbrowser::open(&format!("http://{}:{}", host, port)) {
            eprintln!("Failed to open browser: {}", e);
        }
    }

    while let Ok((stream, _addr)) = listener.accept().await {
        tokio::spawn(handle_web_connection(stream, workspace.clone()));
    }

    Ok(())
}

pub async fn start_gui_server(workspace: &PathBuf, host: &str, port: u16, enable_api: bool) -> Result<()> {
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr).await?;
    
    println!("🖥️ GUI server started at http://{}:{}", host, port);
    if enable_api {
        println!("🔌 API enabled at http://{}:{}/api", host, port);
    }

    while let Ok((stream, _addr)) = listener.accept().await {
        tokio::spawn(handle_gui_connection(stream, workspace.clone(), enable_api));
    }

    Ok(())
}

async fn handle_web_connection(
    stream: TcpStream,
    workspace: PathBuf,
) -> Result<()> {
    let ws_stream = accept_hdr_async(stream, WebSocketConfig::default()).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Send initial message with workspace info
    let init_msg = serde_json::json!({
        "type": "init",
        "workspace": workspace.to_string_lossy(),
        "version": env!("CARGO_PKG_VERSION")
    });
    
    ws_sender.send(Message::Text(init_msg.to_string())).await?;

    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(response) = handle_web_message(&text, &workspace).await {
                    ws_sender.send(Message::Text(response)).await?;
                }
            }
            Ok(Message::Close(_)) => {
                break;
            }
            Err(e) => {
                eprintln!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

async fn handle_gui_connection(
    stream: TcpStream,
    workspace: PathBuf,
    enable_api: bool,
) -> Result<()> {
    let ws_stream = accept_hdr_async(stream, WebSocketConfig::default()).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Send initial message
    let init_msg = serde_json::json!({
        "type": "init",
        "workspace": workspace.to_string_lossy(),
        "api_enabled": enable_api,
        "version": env!("CARGO_PKG_VERSION")
    });
    
    ws_sender.send(Message::Text(init_msg.to_string())).await?;

    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(response) = handle_gui_message(&text, &workspace, enable_api).await {
                    ws_sender.send(Message::Text(response)).await?;
                }
            }
            Ok(Message::Close(_)) => {
                break;
            }
            Err(e) => {
                eprintln!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

async fn handle_web_message(message: &str, workspace: &PathBuf) -> Result<String> {
    let parsed: serde_json::Value = serde_json::from_str(message)?;
    
    match parsed.get("type").and_then(|v| v.as_str()) {
        Some("ping") => Ok(serde_json::json!({
            "type": "pong",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }).to_string()),
        
        Some("get_workspace") => Ok(serde_json::json!({
            "type": "workspace_info",
            "workspace": workspace.to_string_lossy()
        }).to_string()),
        
        Some("execute_task") => {
            let task = parsed.get("task").and_then(|v| v.as_str()).unwrap_or("");
            // TODO: Execute task
            Ok(serde_json::json!({
                "type": "task_result",
                "task": task,
                "status": "started",
                "message": format!("Task '{}' started", task)
            }).to_string())
        }
        
        _ => Ok(serde_json::json!({
            "type": "error",
            "message": "Unknown message type"
        }).to_string())
    }
}

async fn handle_gui_message(message: &str, workspace: &PathBuf, enable_api: bool) -> Result<String> {
    let parsed: serde_json::Value = serde_json::from_str(message)?;
    
    match parsed.get("type").and_then(|v| v.as_str()) {
        Some("ping") => Ok(serde_json::json!({
            "type": "pong",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }).to_string()),
        
        Some("get_status") => Ok(serde_json::json!({
            "type": "status",
            "status": "running",
            "workspace": workspace.to_string_lossy(),
            "api_enabled": enable_api
        }).to_string()),
        
        Some("start_agent") => {
            let task = parsed.get("task").and_then(|v| v.as_str()).unwrap_or("Interactive mode");
            // TODO: Start agent with task
            Ok(serde_json::json!({
                "type": "agent_started",
                "task": task,
                "status": "running"
            }).to_string())
        }
        
        _ => Ok(serde_json::json!({
            "type": "error",
            "message": "Unknown message type"
        }).to_string())
    }
}
