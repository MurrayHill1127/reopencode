use axum::{Json, extract::Query, http::StatusCode};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct FindMatch {
    pub file: String,
    pub line: u32,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct FindFileResult {
    pub path: String,
    pub file_type: String,
}

#[derive(Debug, Deserialize)]
pub struct FindQuery {
    pub pattern: String,
}

#[derive(Debug, Deserialize)]
pub struct FindFileQuery {
    pub query: String,
}

/// GET /find?pattern=... - Search for text in files
pub async fn find(Query(query): Query<FindQuery>) -> Result<Json<Vec<FindMatch>>, StatusCode> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    let results = search_with_ripgrep(&cwd, &query.pattern)
        .or_else(|| search_with_grep(&cwd, &query.pattern))
        .unwrap_or_default();

    Ok(Json(results))
}

/// GET /find/file?query=... - Find files by name
pub async fn find_file(
    Query(query): Query<FindFileQuery>,
) -> Result<Json<Vec<FindFileResult>>, StatusCode> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    let results = find_files_by_name(&cwd, &query.query);
    Ok(Json(results))
}

fn search_with_ripgrep(dir: &Path, pattern: &str) -> Option<Vec<FindMatch>> {
    let output = std::process::Command::new("rg")
        .args(["-n", "--no-heading", pattern])
        .current_dir(dir)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Some(parse_grep_output(&stdout))
}

fn search_with_grep(dir: &Path, pattern: &str) -> Option<Vec<FindMatch>> {
    let output = std::process::Command::new("grep")
        .args(["-rn", pattern, "."])
        .current_dir(dir)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Some(parse_grep_output(&stdout))
}

fn parse_grep_output(output: &str) -> Vec<FindMatch> {
    output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(3, ':').collect();
            if parts.len() >= 3 {
                let line_num = parts[1].parse::<u32>().ok()?;
                Some(FindMatch {
                    file: parts[0].to_string(),
                    line: line_num,
                    content: parts[2].to_string(),
                })
            } else {
                None
            }
        })
        .collect()
}

fn find_files_by_name(dir: &Path, query: &str) -> Vec<FindFileResult> {
    let mut results = Vec::new();

    for entry in walkdir::WalkDir::new(dir)
        .max_depth(5)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .to_lowercase()
                .contains(&query.to_lowercase())
        })
    {
        let path = entry.path().display().to_string();
        let file_type = if entry.file_type().is_dir() {
            "directory"
        } else {
            "file"
        };
        results.push(FindFileResult {
            path,
            file_type: file_type.to_string(),
        });
    }

    results
}
