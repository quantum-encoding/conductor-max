// 🔱 Conductor Max - AI Orchestration Platform
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use tauri::{Manager, WebviewWindowBuilder};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod orchestrator;
use orchestrator::{AgentOrchestrator, AgentConfig, AgentType};

#[derive(Clone)]
struct AppState {
    orchestrator: Arc<AgentOrchestrator>,
}

#[tauri::command]
async fn spawn_agent(
    state: tauri::State<'_, AppState>,
    agent_type: String,
    api_key: String,
    agent_id: Option<String>,
    workspace_path: Option<String>,
) -> Result<String, String> {
    let agent_type = match agent_type.as_str() {
        "claude" => AgentType::Claude,
        "gemini" => AgentType::Gemini,
        _ => return Err(format!("Unknown agent type: {}", agent_type)),
    };

    let config = AgentConfig {
        agent_type,
        api_key,
        agent_id: agent_id.clone(),
        workspace_path,
    };

    state.orchestrator
        .spawn_agent(config)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn send_to_agent(
    state: tauri::State<'_, AppState>,
    agent_id: String,
    command: String,
) -> Result<(), String> {
    state.orchestrator
        .send_command(&agent_id, &command)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn send_raw_to_agent(
    state: tauri::State<'_, AppState>,
    agent_id: String,
    data: Vec<u8>,
) -> Result<(), String> {
    let agent = state.orchestrator.agents.get(&agent_id)
        .ok_or_else(|| format!("Agent {} not found", agent_id))?;
    
    agent.send_raw(&data).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_agent_output(
    state: tauri::State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<u8>, String> {
    let agent = state.orchestrator.agents.get(&agent_id)
        .ok_or_else(|| format!("Agent {} not found", agent_id))?;
    
    match agent.get_output().await {
        Some(data) => Ok(data),
        None => Ok(Vec::new()),
    }
}

#[tauri::command]
async fn resize_agent_terminal(
    state: tauri::State<'_, AppState>,
    agent_id: String,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    let agent = state.orchestrator.agents.get(&agent_id)
        .ok_or_else(|| format!("Agent {} not found", agent_id))?;
    
    agent.resize(rows, cols).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn kill_agent(
    state: tauri::State<'_, AppState>,
    agent_id: String,
) -> Result<(), String> {
    state.orchestrator
        .kill_agent(&agent_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_agent_status(
    state: tauri::State<'_, AppState>,
    agent_id: String,
) -> Result<serde_json::Value, String> {
    state.orchestrator
        .get_agent_status(&agent_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_agents(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    Ok(state.orchestrator.list_agents().await)
}

#[tauri::command]
async fn open_strategy_window(
    app: tauri::AppHandle,
) -> Result<(), String> {
    // Check if strategy window already exists
    if app.get_webview_window("strategy").is_some() {
        // Focus existing window
        if let Some(window) = app.get_webview_window("strategy") {
            window.set_focus().map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    
    // Create new strategy window
    let _window = WebviewWindowBuilder::new(&app, "strategy", 
        tauri::WebviewUrl::App("strategy.html".into()))
        .title("🎯 Strategy AI - Conductor Max")
        .inner_size(800.0, 600.0)
        .position(100.0, 100.0)
        .resizable(true)
        .build()
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
async fn open_agent_window(
    app: tauri::AppHandle,
    agent_id: String,
    agent_type: String,
) -> Result<(), String> {
    let window_id = format!("agent_{}", agent_id);
    
    // Check if window already exists
    if app.get_webview_window(&window_id).is_some() {
        // Focus existing window
        if let Some(window) = app.get_webview_window(&window_id) {
            window.set_focus().map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    
    // Create new agent window
    let url = format!("agent.html?id={}&type={}", agent_id, agent_type);
    let _window = WebviewWindowBuilder::new(&app, &window_id, 
        tauri::WebviewUrl::App(url.into()))
        .title(format!("🤖 {} Agent - {}", agent_type.to_uppercase(), &agent_id[..8]))
        .inner_size(1024.0, 768.0)
        .resizable(true)
        .build()
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "conductor_max=debug,tauri=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🔱 Starting Conductor Max...");

    let orchestrator = Arc::new(AgentOrchestrator::new());
    let app_state = AppState { orchestrator };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            spawn_agent,
            send_to_agent,
            send_raw_to_agent,
            get_agent_output,
            resize_agent_terminal,
            kill_agent,
            get_agent_status,
            list_agents,
            open_strategy_window,
            open_agent_window,
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            
            // Set up window event handlers
            let _app_handle = app.handle().clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    info!("Window close requested, cleaning up agents...");
                    // Cleanup will be handled by orchestrator Drop impl
                }
            });

            info!("✨ Conductor Max initialized successfully");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Conductor Max");
}