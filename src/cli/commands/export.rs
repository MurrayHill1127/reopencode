//! Export session as JSON

use anyhow::{Context, Result, bail};
use inquire::Select;

use crate::storage::{BackendType, Storage};

/// Run export command
pub async fn run(session_id: Option<String>) -> Result<()> {
    let storage = Storage::new(BackendType::Json).await?;

    let sid = match session_id {
        Some(id) => id,
        None => select_session(&storage).await?,
    };

    let session = storage
        .sessions()
        .get(&sid)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Session not found: {}", sid))?;

    let messages = storage
        .messages()
        .list(&sid)
        .await
        .context("Failed to list messages")?;

    let export = serde_json::json!({
        "info": session,
        "messages": messages.iter().map(|m| serde_json::json!({
            "info": m.info,
            "parts": m.parts
        })).collect::<Vec<_>>()
    });

    println!("{}", serde_json::to_string_pretty(&export)?);
    Ok(())
}

/// Interactive session selection
async fn select_session(storage: &Storage) -> Result<String> {
    let sessions = storage
        .sessions()
        .list(Default::default())
        .await
        .context("Failed to list sessions")?;

    if sessions.is_empty() {
        bail!("No sessions found");
    }

    let options: Vec<_> = sessions
        .iter()
        .map(|s| {
            let updated = chrono::DateTime::from_timestamp_millis(s.time_updated)
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_default();
            let id_suffix = &s.id[s.id.len().saturating_sub(8)..];
            format!("{} • {} ...{}", s.title, updated, id_suffix)
        })
        .collect();

    eprintln!("Select session to export:");
    let selected = Select::new("Session:", options).prompt()?;

    // Extract session ID from selection
    sessions
        .iter()
        .find(|s| {
            let id_suffix = &s.id[s.id.len().saturating_sub(8)..];
            selected.ends_with(id_suffix)
        })
        .map(|s| s.id.clone())
        .ok_or_else(|| anyhow::anyhow!("Invalid selection"))
}
