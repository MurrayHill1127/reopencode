use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Standard flags passed to every git invocation.
const CFG_FLAGS: &[&str] = &[
    "--no-optional-locks",
    "-c",
    "core.autocrlf=false",
    "-c",
    "core.fsmonitor=false",
    "-c",
    "core.longpaths=true",
    "-c",
    "core.symlinks=true",
    "-c",
    "core.quotepath=false",
];

pub struct GitOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: i32,
    pub truncated: bool,
}

impl GitOutput {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).trim().to_string()
    }

    pub fn lines(&self) -> Vec<String> {
        self.text()
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect()
    }
}

/// Run a git command in `cwd`, optionally capping output at `max_bytes`.
pub async fn run(args: &[&str], cwd: &Path, max_bytes: Option<usize>) -> Result<GitOutput> {
    let mut cmd = Command::new("git");
    cmd.args(CFG_FLAGS).args(args).current_dir(cwd);

    let output = cmd.output().await?;
    let exit_code = output.status.code().unwrap_or(1);

    let (stdout, truncated) = match max_bytes {
        Some(limit) if output.stdout.len() > limit => {
            (output.stdout[..limit].to_vec(), true)
        }
        _ => (output.stdout, false),
    };

    Ok(GitOutput {
        stdout,
        stderr: output.stderr,
        exit_code,
        truncated,
    })
}

/// Return the current branch name, or None if detached/no HEAD.
pub async fn branch(cwd: &Path) -> Result<Option<String>> {
    let out = run(
        &["symbolic-ref", "--quiet", "--short", "HEAD"],
        cwd,
        None,
    )
    .await?;
    if out.exit_code != 0 {
        return Ok(None);
    }
    let name = out.text();
    Ok(if name.is_empty() { None } else { Some(name) })
}

/// Return the path prefix of `cwd` relative to the git root.
pub async fn prefix(cwd: &Path) -> Result<String> {
    let out = run(&["rev-parse", "--show-prefix"], cwd, None).await?;
    if out.exit_code != 0 {
        return Ok(String::new());
    }
    Ok(out.text())
}

/// Find the git repository root from `cwd`.
pub async fn root(cwd: &Path) -> Result<PathBuf> {
    let out = run(&["rev-parse", "--show-toplevel"], cwd, None).await?;
    if out.exit_code != 0 {
        return Err(anyhow!("not inside a git repository"));
    }
    Ok(PathBuf::from(out.text()))
}

/// Return true if the repo has at least one commit.
pub async fn has_head(cwd: &Path) -> Result<bool> {
    let out = run(&["rev-parse", "--verify", "HEAD"], cwd, None).await?;
    Ok(out.exit_code == 0)
}

/// Find the merge-base between `base` and `head` (defaults to "HEAD").
pub async fn merge_base(cwd: &Path, base: &str, head: &str) -> Result<Option<String>> {
    let out = run(&["merge-base", base, head], cwd, None).await?;
    if out.exit_code != 0 {
        return Ok(None);
    }
    let sha = out.text();
    Ok(if sha.is_empty() { None } else { Some(sha) })
}

/// Show the contents of `file` at `git_ref`.
pub async fn show(cwd: &Path, git_ref: &str, file: &str, prefix: &str) -> Result<String> {
    let target = if prefix.is_empty() {
        file.to_string()
    } else {
        format!("{}{}", prefix, file)
    };
    let refspec = format!("{}:{}", git_ref, target);
    let out = run(&["show", &refspec], cwd, None).await?;
    if out.exit_code != 0 {
        return Ok(String::new());
    }
    // Binary files contain NUL bytes — return empty for those.
    if out.stdout.contains(&0u8) {
        return Ok(String::new());
    }
    Ok(out.text())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    Added,
    Deleted,
    Modified,
}

#[derive(Debug, Clone)]
pub struct StatusItem {
    pub file: String,
    pub code: String,
    pub status: FileStatus,
}

fn parse_kind(code: &str) -> FileStatus {
    if code == "??" {
        return FileStatus::Added;
    }
    if code.contains('U') {
        return FileStatus::Modified;
    }
    if code.contains('A') && !code.contains('D') {
        return FileStatus::Added;
    }
    if code.contains('D') && !code.contains('A') {
        return FileStatus::Deleted;
    }
    FileStatus::Modified
}

/// Working-tree status (staged + unstaged, including untracked).
pub async fn status(cwd: &Path) -> Result<Vec<StatusItem>> {
    let out = run(
        &[
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
            "--no-renames",
            "-z",
            "--",
            ".",
        ],
        cwd,
        None,
    )
    .await?;

    let text = String::from_utf8_lossy(&out.stdout);
    let items: Vec<StatusItem> = text
        .split('\0')
        .filter(|s| !s.is_empty())
        .filter_map(|entry| {
            if entry.len() < 3 {
                return None;
            }
            let file = entry[3..].to_string();
            if file.is_empty() {
                return None;
            }
            let code = entry[..2].to_string();
            let status = parse_kind(&code);
            Some(StatusItem { file, code, status })
        })
        .collect();

    Ok(items)
}

/// Files changed between `git_ref` and the working tree.
pub async fn diff(cwd: &Path, git_ref: &str) -> Result<Vec<StatusItem>> {
    let out = run(
        &[
            "diff",
            "--no-ext-diff",
            "--no-renames",
            "--name-status",
            "-z",
            git_ref,
            "--",
            ".",
        ],
        cwd,
        None,
    )
    .await?;

    let text = String::from_utf8_lossy(&out.stdout);
    let parts: Vec<&str> = text.split('\0').filter(|s| !s.is_empty()).collect();

    let mut items = Vec::new();
    let mut i = 0;
    while i + 1 < parts.len() {
        let code = parts[i].to_string();
        let file = parts[i + 1].to_string();
        if !code.is_empty() && !file.is_empty() {
            let status = parse_kind(&code);
            items.push(StatusItem { file, code, status });
        }
        i += 2;
    }

    Ok(items)
}

#[derive(Debug, Clone)]
pub struct StatItem {
    pub file: String,
    pub additions: u32,
    pub deletions: u32,
}

/// Line-count stats (additions / deletions) for each changed file.
pub async fn stats(cwd: &Path, git_ref: &str) -> Result<Vec<StatItem>> {
    let out = run(
        &[
            "diff",
            "--no-ext-diff",
            "--no-renames",
            "--numstat",
            "-z",
            git_ref,
            "--",
            ".",
        ],
        cwd,
        None,
    )
    .await?;

    let text = String::from_utf8_lossy(&out.stdout);
    let items: Vec<StatItem> = text
        .split('\0')
        .filter(|s| !s.is_empty())
        .filter_map(|entry| {
            let mut parts = entry.splitn(3, '\t');
            let adds = parts.next()?.trim();
            let dels = parts.next()?.trim();
            let file = parts.next()?.to_string();
            if file.is_empty() {
                return None;
            }
            let additions = if adds == "-" { 0 } else { adds.parse().unwrap_or(0) };
            let deletions = if dels == "-" { 0 } else { dels.parse().unwrap_or(0) };
            Some(StatItem { file, additions, deletions })
        })
        .collect();

    Ok(items)
}

#[derive(Debug, Clone)]
pub struct Patch {
    pub text: String,
    pub truncated: bool,
}

/// Unified diff for a single tracked file against `git_ref`.
pub async fn patch(
    cwd: &Path,
    git_ref: &str,
    file: &str,
    context: Option<u32>,
    max_bytes: Option<usize>,
) -> Result<Patch> {
    let unified = format!("--unified={}", context.unwrap_or(3));
    let out = run(
        &[
            "diff",
            "--patch",
            "--no-ext-diff",
            "--no-renames",
            &unified,
            git_ref,
            "--",
            file,
        ],
        cwd,
        max_bytes,
    )
    .await?;

    if out.truncated {
        return Ok(Patch { text: String::new(), truncated: true });
    }
    Ok(Patch { text: out.text(), truncated: false })
}

/// Unified diff for all changes against `git_ref`.
pub async fn patch_all(
    cwd: &Path,
    git_ref: &str,
    context: Option<u32>,
    max_bytes: Option<usize>,
) -> Result<Patch> {
    let unified = format!("--unified={}", context.unwrap_or(3));
    let out = run(
        &[
            "diff",
            "--patch",
            "--no-ext-diff",
            "--no-renames",
            &unified,
            git_ref,
            "--",
            ".",
        ],
        cwd,
        max_bytes,
    )
    .await?;
    Ok(Patch { text: out.text(), truncated: out.truncated })
}

/// Unified diff for an untracked file (diff against /dev/null).
pub async fn patch_untracked(
    cwd: &Path,
    file: &str,
    context: Option<u32>,
    max_bytes: Option<usize>,
) -> Result<Patch> {
    let unified = format!("--unified={}", context.unwrap_or(3));
    let out = run(
        &[
            "diff",
            "--no-index",
            "--patch",
            "--no-ext-diff",
            "--no-renames",
            &unified,
            "--",
            "/dev/null",
            file,
        ],
        cwd,
        max_bytes,
    )
    .await?;
    if out.truncated {
        return Ok(Patch { text: String::new(), truncated: true });
    }
    Ok(Patch { text: out.text(), truncated: false })
}

/// numstat for a single untracked file (diff against /dev/null).
pub async fn stat_untracked(cwd: &Path, file: &str) -> Result<Option<StatItem>> {
    let out = run(
        &["diff", "--no-index", "--numstat", "--", "/dev/null", file],
        cwd,
        Some(4096),
    )
    .await?;

    if out.truncated {
        return Ok(None);
    }

    let text = out.text();
    let mut parts = text.splitn(3, '\t');
    let adds = parts.next().unwrap_or("").trim();
    let dels = parts.next().unwrap_or("").trim();

    let additions: u32 = if adds == "-" { 0 } else { adds.parse().unwrap_or(0) };
    let deletions: u32 = if dels == "-" { 0 } else { dels.parse().unwrap_or(0) };

    Ok(Some(StatItem { file: file.to_string(), additions, deletions }))
}

/// Detect the default branch (main / master / configured).
pub async fn default_branch(cwd: &Path) -> Result<Option<(String, String)>> {
    // Try origin/HEAD first.
    let remotes_out = run(&["remote"], cwd, None).await?;
    let remotes = remotes_out.lines();
    let remote = if remotes.contains(&"origin".to_string()) {
        "origin"
    } else if !remotes.is_empty() {
        &remotes[0]
    } else {
        ""
    };

    if !remote.is_empty() {
        let symref = format!("refs/remotes/{}/HEAD", remote);
        let head_out = run(&["symbolic-ref", &symref], cwd, None).await?;
        if head_out.exit_code == 0 {
            let full_ref = head_out.text();
            let prefix = format!("refs/remotes/{}/", remote);
            let name = full_ref
                .strip_prefix(&prefix)
                .unwrap_or("")
                .to_string();
            if !name.is_empty() {
                return Ok(Some((name, full_ref)));
            }
        }
    }

    // Fall back to local branches.
    let refs_out = run(
        &["for-each-ref", "--format=%(refname:short)", "refs/heads"],
        cwd,
        None,
    )
    .await?;
    let local: Vec<String> = refs_out.lines();

    // Check configured init.defaultBranch.
    let cfg_out = run(&["config", "init.defaultBranch"], cwd, None).await?;
    if cfg_out.exit_code == 0 {
        let name = cfg_out.text();
        if !name.is_empty() && local.contains(&name) {
            return Ok(Some((name.clone(), name)));
        }
    }

    for candidate in ["main", "master"] {
        if local.contains(&candidate.to_string()) {
            return Ok(Some((candidate.to_string(), candidate.to_string())));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command as StdCommand;
    use tempfile::tempdir;

    async fn init_repo(dir: &Path) {
        StdCommand::new("git").args(["init"]).current_dir(dir).output().unwrap();
        StdCommand::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir)
            .output()
            .unwrap();
        StdCommand::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir)
            .output()
            .unwrap();
    }

    async fn commit_file(dir: &Path, name: &str, content: &str) {
        std::fs::write(dir.join(name), content).unwrap();
        StdCommand::new("git").args(["add", "."]).current_dir(dir).output().unwrap();
        StdCommand::new("git")
            .args(["commit", "-m", "add file"])
            .current_dir(dir)
            .output()
            .unwrap();
    }

    #[tokio::test]
    async fn test_has_head_empty_repo() {
        let dir = tempdir().unwrap();
        init_repo(dir.path()).await;
        assert!(!has_head(dir.path()).await.unwrap());
    }

    #[tokio::test]
    async fn test_has_head_with_commit() {
        let dir = tempdir().unwrap();
        init_repo(dir.path()).await;
        commit_file(dir.path(), "README.md", "hello").await;
        assert!(has_head(dir.path()).await.unwrap());
    }

    #[tokio::test]
    async fn test_branch() {
        let dir = tempdir().unwrap();
        init_repo(dir.path()).await;
        commit_file(dir.path(), "f.txt", "x").await;
        let b = branch(dir.path()).await.unwrap();
        assert!(b.is_some());
    }

    #[tokio::test]
    async fn test_root() {
        let dir = tempdir().unwrap();
        init_repo(dir.path()).await;
        let r = root(dir.path()).await.unwrap();
        // tempdir path may differ in case on macOS — compare canonical forms.
        assert_eq!(
            r.canonicalize().unwrap(),
            dir.path().canonicalize().unwrap()
        );
    }

    #[tokio::test]
    async fn test_status_clean() {
        let dir = tempdir().unwrap();
        init_repo(dir.path()).await;
        commit_file(dir.path(), "f.txt", "hello").await;
        let s = status(dir.path()).await.unwrap();
        assert!(s.is_empty());
    }

    #[tokio::test]
    async fn test_status_modified() {
        let dir = tempdir().unwrap();
        init_repo(dir.path()).await;
        commit_file(dir.path(), "f.txt", "hello").await;
        std::fs::write(dir.path().join("f.txt"), "world").unwrap();
        let s = status(dir.path()).await.unwrap();
        assert!(!s.is_empty());
        assert_eq!(s[0].status, FileStatus::Modified);
    }

    #[tokio::test]
    async fn test_show_file() {
        let dir = tempdir().unwrap();
        init_repo(dir.path()).await;
        commit_file(dir.path(), "hello.txt", "world content").await;
        let content = show(dir.path(), "HEAD", "hello.txt", "").await.unwrap();
        assert_eq!(content.trim(), "world content");
    }

    #[tokio::test]
    async fn test_stats() {
        let dir = tempdir().unwrap();
        init_repo(dir.path()).await;
        commit_file(dir.path(), "f.txt", "line1\n").await;
        std::fs::write(dir.path().join("f.txt"), "line1\nline2\n").unwrap();
        StdCommand::new("git").args(["add", "."]).current_dir(dir.path()).output().unwrap();
        let s = stats(dir.path(), "HEAD").await.unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].additions, 1);
        assert_eq!(s[0].deletions, 0);
    }

    #[tokio::test]
    async fn test_patch() {
        let dir = tempdir().unwrap();
        init_repo(dir.path()).await;
        commit_file(dir.path(), "f.txt", "old\n").await;
        std::fs::write(dir.path().join("f.txt"), "new\n").unwrap();
        let p = patch(dir.path(), "HEAD", "f.txt", None, None).await.unwrap();
        assert!(!p.text.is_empty());
        assert!(p.text.contains("-old"));
        assert!(p.text.contains("+new"));
    }
}
