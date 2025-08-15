// 🔱 Conductor Max - AI Orchestration Platform
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use tauri::Manager;
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
        workspace_path: None,
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
async fn get_agent_output(
    state: tauri::State<'_, AppState>,
    agent_id: String,
    lines: Option<usize>,
) -> Result<Vec<String>, String> {
    state.orchestrator
        .get_agent_output(&agent_id, lines.unwrap_or(100))
        .await
        .map_err(|e| e.to_string())
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
            kill_agent,
            get_agent_status,
            list_agents,
            get_agent_output,
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            
            // Set up window event handlers
            let app_handle = app.handle().clone();
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