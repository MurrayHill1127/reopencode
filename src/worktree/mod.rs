//! Git worktree management.
//!
//! Mirrors the key paths from opencode's worktree/index.ts:
//!   create()  — git worktree add -b <branch> <dir>
//!   remove()  — git worktree remove --force + branch -D
//!   reset()   — git reset --hard + git clean -ffdx
//!   list()    — git worktree list --porcelain
//!
//! Worktrees are stored under `{data_dir}/roc/worktree/{project_id}/{name}`.

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use crate::git;

// ─── types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub name: String,
    pub branch: String,
    pub directory: PathBuf,
}

// ─── internal helpers ────────────────────────────────────────────────────────

fn slugify(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn short_id() -> String {
    let uuid = crate::util::generate_uuid();
    uuid.split('-').next().unwrap_or("xxxx").to_string()
}

/// Parse `git worktree list --porcelain` output into (path, branch) pairs.
fn parse_worktree_list(text: &str) -> Vec<(String, Option<String>)> {
    let mut result = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_branch: Option<String> = None;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            if let Some(path) = current_path.take() {
                result.push((path, current_branch.take()));
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("worktree ") {
            if let Some(path) = current_path.take() {
                result.push((path, current_branch.take()));
            }
            current_path = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("branch ") {
            current_branch = Some(rest.trim().to_string());
        }
    }
    if let Some(path) = current_path {
        result.push((path, current_branch));
    }
    result
}

/// Find the entry whose path matches `directory` (case-insensitive on Windows).
fn find_entry(
    entries: &[(String, Option<String>)],
    directory: &Path,
) -> Option<(String, Option<String>)> {
    let target = std::fs::canonicalize(directory)
        .unwrap_or_else(|_| directory.to_path_buf())
        .to_string_lossy()
        .to_lowercase();

    for (path, branch) in entries {
        let canonical = std::fs::canonicalize(path)
            .unwrap_or_else(|_| PathBuf::from(path))
            .to_string_lossy()
            .to_lowercase();
        if canonical == target {
            return Some((path.clone(), branch.clone()));
        }
    }
    None
}

// ─── public API ──────────────────────────────────────────────────────────────

/// Return the default root directory for worktrees of a given project.
pub fn worktree_root(project_id: &str) -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("roc")
        .join("worktree")
        .join(project_id)
}

/// Determine a unique worktree name/branch/directory.
///
/// Tries `name` first (or a random slug), then appends a short random suffix
/// if the name is already taken.
pub async fn make_info(root: &Path, name: Option<&str>, repo: &Path) -> Result<WorktreeInfo> {
    const MAX_ATTEMPTS: usize = 26;
    let base = name.map(slugify).filter(|s| !s.is_empty());

    for attempt in 0..MAX_ATTEMPTS {
        let candidate = match &base {
            Some(b) => {
                if attempt == 0 {
                    b.clone()
                } else {
                    format!("{}-{}", b, short_id())
                }
            }
            None => short_id(),
        };

        let branch = format!("opencode/{}", candidate);
        let dir = root.join(&candidate);

        if dir.exists() {
            continue;
        }

        // Make sure the branch doesn't already exist.
        let check = git::run(
            &["show-ref", "--verify", "--quiet", &format!("refs/heads/{}", branch)],
            repo,
            None,
        )
        .await?;
        if check.exit_code == 0 {
            continue;
        }

        return Ok(WorktreeInfo {
            name: candidate,
            branch,
            directory: dir,
        });
    }

    bail!("Failed to generate a unique worktree name after {} attempts", MAX_ATTEMPTS)
}

/// Create a git worktree at `info.directory` on branch `info.branch`.
pub async fn create(info: &WorktreeInfo, repo: &Path) -> Result<()> {
    std::fs::create_dir_all(
        info.directory
            .parent()
            .unwrap_or(Path::new(".")),
    )?;

    let result = git::run(
        &[
            "worktree",
            "add",
            "--no-checkout",
            "-b",
            &info.branch,
            info.directory.to_str().unwrap_or(""),
        ],
        repo,
        None,
    )
    .await?;

    if result.exit_code != 0 {
        bail!(
            "git worktree add failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    // Populate the working tree.
    let reset = git::run(&["reset", "--hard"], &info.directory, None).await?;
    if reset.exit_code != 0 {
        bail!(
            "git reset --hard failed in worktree: {}",
            String::from_utf8_lossy(&reset.stderr)
        );
    }

    Ok(())
}

/// Remove a git worktree by its directory path.
///
/// Returns `false` if the directory didn't correspond to a tracked worktree
/// (the directory is still removed if it exists).
pub async fn remove(directory: &Path, repo: &Path) -> Result<bool> {
    let list_out = git::run(&["worktree", "list", "--porcelain"], repo, None).await?;
    if list_out.exit_code != 0 {
        bail!(
            "git worktree list failed: {}",
            String::from_utf8_lossy(&list_out.stderr)
        );
    }

    let entries = parse_worktree_list(&list_out.text());
    let entry = find_entry(&entries, directory);

    if entry.is_none() {
        // Directory not tracked as a worktree — clean up the path directly.
        if directory.exists() {
            tokio::fs::remove_dir_all(directory).await?;
        }
        return Ok(false);
    }

    let (path, branch) = entry.unwrap();

    let rm = git::run(
        &["worktree", "remove", "--force", &path],
        repo,
        None,
    )
    .await?;

    if rm.exit_code != 0 {
        // Double-check: if the entry is now gone, treat as success.
        let list2 = git::run(&["worktree", "list", "--porcelain"], repo, None).await?;
        let entries2 = parse_worktree_list(&list2.text());
        if find_entry(&entries2, directory).is_some() {
            bail!(
                "git worktree remove failed: {}",
                String::from_utf8_lossy(&rm.stderr)
            );
        }
    }

    // Remove the directory if still present.
    if directory.exists() {
        let _ = tokio::fs::remove_dir_all(directory).await;
    }

    // Delete the branch.
    if let Some(b) = branch {
        let branch_name = b.strip_prefix("refs/heads/").unwrap_or(&b);
        let del = git::run(&["branch", "-D", branch_name], repo, None).await?;
        if del.exit_code != 0 {
            bail!(
                "git branch -D {} failed: {}",
                branch_name,
                String::from_utf8_lossy(&del.stderr)
            );
        }
    }

    Ok(true)
}

/// Reset a worktree to a clean state (HEAD + no untracked files).
pub async fn reset(directory: &Path) -> Result<()> {
    let hard = git::run(&["reset", "--hard"], directory, None).await?;
    if hard.exit_code != 0 {
        bail!(
            "git reset --hard failed: {}",
            String::from_utf8_lossy(&hard.stderr)
        );
    }

    let clean = git::run(&["clean", "-ffdx"], directory, None).await?;
    if clean.exit_code != 0 {
        bail!(
            "git clean -ffdx failed: {}",
            String::from_utf8_lossy(&clean.stderr)
        );
    }

    Ok(())
}

/// List all worktrees for the repository at `repo`.
pub async fn list(repo: &Path) -> Result<Vec<WorktreeInfo>> {
    let out = git::run(&["worktree", "list", "--porcelain"], repo, None).await?;
    if out.exit_code != 0 {
        bail!(
            "git worktree list failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    let entries = parse_worktree_list(&out.text());
    let infos = entries
        .into_iter()
        .filter_map(|(path, branch)| {
            let dir = PathBuf::from(&path);
            let branch = branch?;
            let name = branch
                .strip_prefix("refs/heads/opencode/")
                .unwrap_or(dir.file_name()?.to_str()?)
                .to_string();
            Some(WorktreeInfo {
                name,
                branch: branch.strip_prefix("refs/heads/").unwrap_or(&branch).to_string(),
                directory: dir,
            })
        })
        .collect();

    Ok(infos)
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World!"), "hello-world");
        assert_eq!(slugify("  foo--bar  "), "foo-bar");
        assert_eq!(slugify("abc123"), "abc123");
    }

    #[test]
    fn test_parse_worktree_list() {
        let input = "\
worktree /home/user/project
HEAD abc123
branch refs/heads/main

worktree /home/user/project/wt1
HEAD def456
branch refs/heads/opencode/feature-1

";
        let entries = parse_worktree_list(input);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, "/home/user/project");
        assert_eq!(entries[0].1.as_deref(), Some("refs/heads/main"));
        assert_eq!(entries[1].0, "/home/user/project/wt1");
        assert_eq!(entries[1].1.as_deref(), Some("refs/heads/opencode/feature-1"));
    }

    #[test]
    fn test_parse_worktree_list_bare() {
        let input = "\
worktree /tmp/repo
HEAD abcdef
bare

";
        let entries = parse_worktree_list(input);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].1.is_none());
    }

    #[test]
    fn test_worktree_root() {
        let root = worktree_root("proj_abc");
        assert!(root.to_string_lossy().contains("roc"));
        assert!(root.to_string_lossy().contains("worktree"));
        assert!(root.to_string_lossy().contains("proj_abc"));
    }
}
