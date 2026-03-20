//! Import session from JSON file or share URL

use anyhow::{Context, Result, bail};
use regex::Regex;

use crate::storage::{
    BackendType, MessageInfo, MessagePart, MessageTime, MessageWithParts, SessionRecord, Storage,
};

/// Share API response item
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ShareData {
    Session { data: SessionRecord },
    Message { data: ImportMessage },
    Part { data: ImportPart },
}

/// Message from import
#[derive(Debug, Clone, serde::Deserialize)]
struct ImportMessage {
    id: String,
    #[serde(rename = "sessionID")]
    session_id: String,
    role: String,
    agent: Option<String>,
    time: Option<ImportTime>,
}

/// Part from import
#[derive(Debug, Clone, serde::Deserialize)]
struct ImportPart {
    id: String,
    #[serde(rename = "messageID")]
    message_id: String,
    #[serde(flatten)]
    data: serde_json::Value,
}

/// Time info from import
#[derive(Debug, Clone, serde::Deserialize)]
struct ImportTime {
    created: i64,
}

/// Export format from file
#[derive(Debug, Clone, serde::Deserialize)]
struct ExportData {
    info: SessionRecord,
    messages: Vec<MessageWithParts>,
}

fn parse_share_url(url: &str) -> Option<String> {
    let re = Regex::new(r"^https?://[^/]+/share/([a-zA-Z0-9_-]+)$").ok()?;
    re.captures(url)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

fn extract_base_url(url: &str) -> Option<String> {
    let re = Regex::new(r"^(https?://[^/]+)").ok()?;
    re.captures(url)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

async fn fetch_share_data(url: &str) -> Result<ExportData> {
    let slug = parse_share_url(url).ok_or_else(|| anyhow::anyhow!("Invalid share URL format"))?;

    let base_url =
        extract_base_url(url).ok_or_else(|| anyhow::anyhow!("Failed to extract base URL"))?;

    // Try new API path first, fallback to old
    let endpoints = vec![
        format!("{}/api/shares/{}/data", base_url, slug),
        format!("{}/api/share/{}/data", base_url, slug),
    ];

    let client = reqwest::Client::new();
    let mut last_error = None;

    for endpoint in endpoints {
        match client.get(&endpoint).send().await {
            Ok(resp) if resp.status().is_success() => {
                let items: Vec<ShareData> =
                    resp.json().await.context("Failed to parse API response")?;
                return transform_share_data(items);
            }
            Ok(resp) => {
                last_error = Some(format!("HTTP {}", resp.status()));
            }
            Err(e) => {
                last_error = Some(e.to_string());
            }
        }
    }

    bail!(
        "Failed to fetch share data: {}",
        last_error.unwrap_or_default()
    )
}

/// Transform flat API response to nested structure
fn transform_share_data(items: Vec<ShareData>) -> Result<ExportData> {
    let mut session: Option<SessionRecord> = None;
    let mut messages: std::collections::HashMap<String, ImportMessage> = Default::default();
    let mut parts: std::collections::HashMap<String, Vec<MessagePart>> = Default::default();

    for item in items {
        match item {
            ShareData::Session { data } => session = Some(data),
            ShareData::Message { data } => {
                messages.insert(data.id.clone(), data.clone());
                parts.entry(data.id.clone()).or_default();
            }
            ShareData::Part { data } => {
                let part: MessagePart = serde_json::from_value(data.data.clone())
                    .context("Failed to parse message part")?;
                parts.entry(data.message_id.clone()).or_default().push(part);
            }
        }
    }

    let session = session.ok_or_else(|| anyhow::anyhow!("No session in share data"))?;

    let msgs: Vec<MessageWithParts> = messages
        .into_values()
        .map(|m| MessageWithParts {
            info: MessageInfo {
                id: m.id.clone(),
                session_id: session.id.clone(),
                role: parse_role(&m.role),
                agent: m.agent,
                time: MessageTime {
                    created: m.time.map(|t| t.created).unwrap_or_default(),
                    updated: None,
                },
            },
            parts: parts.remove(&m.id).unwrap_or_default(),
        })
        .collect();

    Ok(ExportData {
        info: session,
        messages: msgs,
    })
}

fn parse_role(s: &str) -> crate::storage::MessageRole {
    match s.to_lowercase().as_str() {
        "assistant" => crate::storage::MessageRole::Assistant,
        _ => crate::storage::MessageRole::User,
    }
}

/// Run import command
pub async fn run(file: String) -> Result<()> {
    let storage = Storage::new(BackendType::Json).await?;

    let data = if file.starts_with("http://") || file.starts_with("https://") {
        fetch_share_data(&file).await?
    } else {
        let content = std::fs::read_to_string(&file)
            .with_context(|| format!("Failed to read file: {}", file))?;
        serde_json::from_str(&content).with_context(|| "Failed to parse JSON")?
    };

    // Create session
    let session = storage
        .sessions()
        .create(crate::storage::SessionCreateInput {
            project_id: data.info.project_id.clone(),
            workspace_id: data.info.workspace_id.clone(),
            parent_id: data.info.parent_id.clone(),
            slug: data.info.slug.clone(),
            directory: data.info.directory.clone(),
            title: Some(data.info.title.clone()),
        })
        .await
        .context("Failed to create session")?;

    // Create messages
    for msg in data.messages {
        storage
            .messages()
            .create(&session.id, msg.info, msg.parts)
            .await
            .context("Failed to create message")?;
    }

    println!("Imported session: {}", session.id);
    Ok(())
}
