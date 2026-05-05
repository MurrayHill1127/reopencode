use anyhow::{anyhow, Result};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tokio::sync::Mutex;
use tracing::{info, warn};

const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

const CFG: &[&str] = &[
    "-c", "core.autocrlf=false",
    "-c", "core.longpaths=true",
    "-c", "core.symlinks=true",
];

const QUOTE: &[&str] = &[
    "-c", "core.autocrlf=false",
    "-c", "core.longpaths=true",
    "-c", "core.symlinks=true",
    "-c", "core.quotepath=false",
];

// ─── public types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Patch {
    pub hash: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffStatus {
    Added,
    Deleted,
    Modified,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileDiff {
    pub file: String,
    pub patch: String,
    pub additions: u32,
    pub deletions: u32,
    pub status: Option<DiffStatus>,
}

// ─── internal git output ─────────────────────────────────────────────────────

struct Out {
    code: i32,
    text: String,
    stderr: String,
}

impl Out {
    fn ok(&self) -> bool {
        self.code == 0
    }
}

// ─── manager ─────────────────────────────────────────────────────────────────

pub struct SnapshotManager {
    gitdir: PathBuf,
    worktree: PathBuf,
    lock: Mutex<()>,
}

impl SnapshotManager {
    /// Create a manager for `worktree`. The shadow git-dir is stored under
    /// the platform data dir so it never pollutes the actual repository.
    pub fn for_worktree(worktree: &Path) -> Self {
        let gitdir = gitdir_for(worktree);
        Self {
            gitdir,
            worktree: worktree.to_path_buf(),
            lock: Mutex::new(()),
        }
    }

    /// Run a git command against the shadow repo (`--git-dir`, `--work-tree`).
    async fn git(&self, extra: &[&str]) -> Out {
        let gitdir = self.gitdir.to_string_lossy().to_string();
        let worktree = self.worktree.to_string_lossy().to_string();

        let mut args: Vec<String> = vec![
            "--git-dir".into(),
            gitdir,
            "--work-tree".into(),
            worktree,
        ];
        args.extend(extra.iter().map(|s| s.to_string()));

        run_git(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>(), &self.worktree).await
    }

    /// Run a git command with extra string-owned args (avoids lifetime issues).
    async fn git_owned(&self, extra: Vec<String>) -> Out {
        let gitdir = self.gitdir.to_string_lossy().to_string();
        let worktree = self.worktree.to_string_lossy().to_string();

        let mut args: Vec<String> = vec![
            "--git-dir".into(),
            gitdir,
            "--work-tree".into(),
            worktree,
        ];
        args.extend(extra);

        run_git(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>(), &self.worktree).await
    }

    /// Check which files in `candidates` are gitignored by the source repo.
    async fn ignored(&self, candidates: &[String]) -> std::collections::HashSet<String> {
        if candidates.is_empty() {
            return Default::default();
        }
        // Build NUL-delimited input for check-ignore --stdin -z
        let stdin_bytes = {
            let mut v = Vec::new();
            for f in candidates {
                v.extend_from_slice(f.as_bytes());
                v.push(0);
            }
            v
        };

        let gitdir_src = self.worktree.join(".git");
        let gitdir_str = gitdir_src.to_string_lossy().to_string();
        let worktree_str = self.worktree.to_string_lossy().to_string();

        let mut cmd = Command::new("git");
        cmd.args(QUOTE)
            .args([
                "--git-dir", &gitdir_str,
                "--work-tree", &worktree_str,
                "check-ignore", "--no-index", "--stdin", "-z",
            ])
            .current_dir(&self.worktree)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(_) => return Default::default(),
        };

        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            let _ = stdin.write_all(&stdin_bytes).await;
        }

        let out = match child.wait_with_output().await {
            Ok(o) => o,
            Err(_) => return Default::default(),
        };

        // exit 0 = some ignored, 1 = none ignored, anything else = error
        if out.status.code().map(|c| c > 1).unwrap_or(true) {
            return Default::default();
        }

        String::from_utf8_lossy(&out.stdout)
            .split('\0')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    /// Remove files from the snapshot index.
    async fn drop_from_index(&self, files: &[String]) {
        if files.is_empty() {
            return;
        }
        let stdin_bytes: Vec<u8> = files.iter().flat_map(|f| {
            let mut v = f.as_bytes().to_vec();
            v.push(0);
            v
        }).collect();

        let gitdir = self.gitdir.to_string_lossy().to_string();
        let worktree = self.worktree.to_string_lossy().to_string();

        let mut cmd = Command::new("git");
        cmd.args(CFG)
            .args([
                "--git-dir", &gitdir,
                "--work-tree", &worktree,
                "rm", "--cached", "-f", "--ignore-unmatch",
                "--pathspec-from-file=-", "--pathspec-file-nul",
            ])
            .current_dir(&self.worktree)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        if let Ok(mut child) = cmd.spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                let _ = stdin.write_all(&stdin_bytes).await;
            }
            let _ = child.wait_with_output().await;
        }
    }

    /// Stage files into the snapshot index.
    async fn stage(&self, files: &[String]) {
        if files.is_empty() {
            return;
        }
        let stdin_bytes: Vec<u8> = files.iter().flat_map(|f| {
            let mut v = f.as_bytes().to_vec();
            v.push(0);
            v
        }).collect();

        let gitdir = self.gitdir.to_string_lossy().to_string();
        let worktree = self.worktree.to_string_lossy().to_string();

        let mut cmd = Command::new("git");
        cmd.args(CFG)
            .args([
                "--git-dir", &gitdir,
                "--work-tree", &worktree,
                "add", "--all", "--sparse",
                "--pathspec-from-file=-", "--pathspec-file-nul",
            ])
            .current_dir(&self.worktree)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        if let Ok(mut child) = cmd.spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                let _ = stdin.write_all(&stdin_bytes).await;
            }
            let out = child.wait_with_output().await;
            if let Ok(o) = out {
                if o.status.code() != Some(0) {
                    warn!(
                        "snapshot stage failed: {}",
                        String::from_utf8_lossy(&o.stderr)
                    );
                }
            }
        }
    }

    /// Sync the exclude file of the shadow repo with the source repo's excludes,
    /// optionally appending extra patterns (for oversized untracked files).
    async fn sync_excludes(&self, extra: &[String]) {
        // Read source repo exclude file.
        let src_exclude = {
            let out = run_git(
                &["rev-parse", "--path-format=absolute", "--git-path", "info/exclude"],
                &self.worktree,
            ).await;
            if out.ok() {
                let p = PathBuf::from(out.text.trim());
                if p.exists() {
                    std::fs::read_to_string(&p).unwrap_or_default()
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        };

        let target = self.gitdir.join("info").join("exclude");
        let _ = tokio::fs::create_dir_all(self.gitdir.join("info")).await;

        let mut lines: Vec<String> = Vec::new();
        let base = src_exclude.trim_end().to_string();
        if !base.is_empty() {
            lines.push(base);
        }
        for item in extra {
            let p = item.replace('\\', "/");
            lines.push(format!("/{}", p));
        }
        let content = if lines.is_empty() {
            String::new()
        } else {
            format!("{}\n", lines.join("\n"))
        };
        let _ = tokio::fs::write(&target, content).await;
    }

    /// Stage all changed / untracked files, respecting gitignore and size limits.
    async fn add(&self) -> Result<()> {
        self.sync_excludes(&[]).await;

        // Modified tracked files in shadow repo.
        let diff_out = self.git(&[
            QUOTE, &[
                "diff-files", "--name-only", "-z", "--", ".",
            ],
        ].concat()).await;

        // Untracked files (not in shadow index).
        let other_out = self.git(&[
            QUOTE, &[
                "ls-files", "--others", "--exclude-standard", "-z", "--", ".",
            ],
        ].concat()).await;

        if !diff_out.ok() || !other_out.ok() {
            warn!(
                "snapshot add: diff-files={} ls-files={}",
                diff_out.code, other_out.code
            );
            return Ok(());
        }

        let tracked: Vec<String> = diff_out.text.split('\0')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let untracked: Vec<String> = other_out.text.split('\0')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let mut all: Vec<String> = {
            let mut seen = std::collections::HashSet::new();
            tracked.iter().chain(untracked.iter())
                .filter(|f| seen.insert((*f).clone()))
                .cloned()
                .collect()
        };

        if all.is_empty() {
            return Ok(());
        }

        let ignored = self.ignored(&all).await;

        if !ignored.is_empty() {
            let ignored_list: Vec<String> = ignored.iter().cloned().collect();
            self.drop_from_index(&ignored_list).await;
        }

        all.retain(|f| !ignored.contains(f));
        if all.is_empty() {
            return Ok(());
        }

        // Find oversized untracked files and exclude them.
        let mut oversized: Vec<String> = Vec::new();
        for f in &untracked {
            if ignored.contains(f) {
                continue;
            }
            let full = self.worktree.join(f);
            if let Ok(meta) = std::fs::metadata(&full) {
                if meta.len() > MAX_FILE_BYTES {
                    oversized.push(f.clone());
                }
            }
        }

        if !oversized.is_empty() {
            self.sync_excludes(&oversized).await;
        }

        let to_stage: Vec<String> = all.into_iter()
            .filter(|f| !oversized.contains(f))
            .collect();

        self.stage(&to_stage).await;
        Ok(())
    }

    /// Initialise the shadow git repo if it doesn't exist yet.
    async fn init_if_needed(&self) -> Result<()> {
        if self.gitdir.exists() {
            return Ok(());
        }
        tokio::fs::create_dir_all(&self.gitdir).await?;

        // Init bare-ish repo with custom git-dir/work-tree.
        let env_gitdir = self.gitdir.to_string_lossy().to_string();
        let env_worktree = self.worktree.to_string_lossy().to_string();
        let out = Command::new("git")
            .arg("init")
            .env("GIT_DIR", &env_gitdir)
            .env("GIT_WORK_TREE", &env_worktree)
            .current_dir(&self.worktree)
            .output()
            .await?;
        if out.status.code() != Some(0) {
            return Err(anyhow!(
                "git init failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }

        // Set required config keys.
        for (key, val) in [
            ("core.autocrlf", "false"),
            ("core.longpaths", "true"),
            ("core.symlinks", "true"),
            ("core.fsmonitor", "false"),
        ] {
            run_git(
                &["--git-dir", &env_gitdir, "config", key, val],
                &self.worktree,
            ).await;
        }

        info!("snapshot repo initialized at {:?}", self.gitdir);
        Ok(())
    }

    // ── public API ────────────────────────────────────────────────────────────

    /// Snapshot the current worktree state.  Returns the tree hash, or `None`
    /// when the worktree is not inside a git repository.
    pub async fn track(&self) -> Result<Option<String>> {
        let _g = self.lock.lock().await;

        // Only snapshot git-tracked worktrees.
        let root_check = run_git(
            &["rev-parse", "--show-toplevel"],
            &self.worktree,
        ).await;
        if !root_check.ok() {
            return Ok(None);
        }

        self.init_if_needed().await?;
        self.add().await?;

        let out = self.git(&["write-tree"]).await;
        if !out.ok() {
            warn!("write-tree failed (code {}): {}", out.code, out.stderr);
            return Ok(None);
        }
        let hash = out.text.trim().to_string();
        if hash.is_empty() {
            return Ok(None);
        }
        info!("snapshot tracked hash={}", hash);
        Ok(Some(hash))
    }

    /// List files that changed since `hash`.
    pub async fn patch(&self, hash: &str) -> Result<Patch> {
        let _g = self.lock.lock().await;

        self.add().await?;

        let out = self.git(&[
            QUOTE, &[
                "diff", "--cached", "--no-ext-diff", "--name-only",
                hash, "--", ".",
            ],
        ].concat()).await;

        if !out.ok() {
            warn!("snapshot patch diff failed (code {})", out.code);
            return Ok(Patch { hash: hash.to_string(), files: vec![] });
        }

        let files: Vec<String> = out.text.trim()
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        let ignored = self.ignored(&files).await;

        let worktree = self.worktree.to_string_lossy().to_string();
        let abs_files: Vec<String> = files.into_iter()
            .filter(|f| !ignored.contains(f))
            .map(|f| {
                let p = self.worktree.join(&f);
                p.to_string_lossy().replace('\\', "/")
            })
            .collect();

        Ok(Patch { hash: hash.to_string(), files: abs_files })
    }

    /// Restore the worktree to the state recorded in `snapshot` (a tree hash).
    pub async fn restore(&self, snapshot: &str) -> Result<()> {
        let _g = self.lock.lock().await;

        let read = self.git(&[
            CFG, &["read-tree", snapshot],
        ].concat()).await;

        if read.ok() {
            let co = self.git(&[
                CFG, &["checkout-index", "-a", "-f"],
            ].concat()).await;
            if !co.ok() {
                warn!("checkout-index failed (code {}): {}", co.code, co.stderr);
            }
        } else {
            warn!("read-tree failed (code {}): {}", read.code, read.stderr);
        }
        Ok(())
    }

    /// Revert specific files listed in `patches` back to their snapshot state.
    pub async fn revert(&self, patches: &[Patch]) -> Result<()> {
        let _g = self.lock.lock().await;

        // Build deduplicated list of (hash, abs_file, rel_file) ops.
        struct Op {
            hash: String,
            file: String,
            rel: String,
        }

        let mut seen = std::collections::HashSet::new();
        let mut ops: Vec<Op> = Vec::new();
        for p in patches {
            for file in &p.files {
                if seen.contains(file) {
                    continue;
                }
                seen.insert(file.clone());
                let rel = {
                    let p = Path::new(file);
                    p.strip_prefix(&self.worktree)
                        .map(|r| r.to_string_lossy().replace('\\', "/"))
                        .unwrap_or_else(|_| file.clone())
                };
                ops.push(Op { hash: p.hash.clone(), file: file.clone(), rel });
            }
        }

        let clash = |a: &str, b: &str| a == b || a.starts_with(&format!("{}/", b)) || b.starts_with(&format!("{}/", a));

        let single = |op: &Op| {
            let hash = op.hash.clone();
            let file = op.file.clone();
            let rel = op.rel.clone();
            let gitdir = self.gitdir.to_string_lossy().to_string();
            let worktree = self.worktree.to_string_lossy().to_string();
            async move {
                let co = run_git(
                    &[CFG, &[
                        "--git-dir", &gitdir, "--work-tree", &worktree,
                        "checkout", &hash, "--", &file,
                    ]].concat(),
                    Path::new(&worktree),
                ).await;
                if co.ok() {
                    return;
                }
                // File may not exist in snapshot — check.
                let ls = run_git(
                    &[CFG, &[
                        "--git-dir", &gitdir, "--work-tree", &worktree,
                        "ls-tree", &hash, "--", &rel,
                    ]].concat(),
                    Path::new(&worktree),
                ).await;
                if ls.ok() && !ls.text.trim().is_empty() {
                    info!("checkout failed but file exists in snapshot, keeping {}", file);
                    return;
                }
                info!("file not in snapshot, deleting {}", file);
                let _ = tokio::fs::remove_file(&file).await;
            }
        };

        let mut i = 0;
        while i < ops.len() {
            let first = &ops[i];
            let mut run_ids: Vec<usize> = vec![i];
            let mut j = i + 1;
            while j < ops.len() && run_ids.len() < 100 {
                let next = &ops[j];
                if next.hash != first.hash {
                    break;
                }
                if run_ids.iter().any(|&id| clash(&ops[id].rel, &next.rel)) {
                    break;
                }
                run_ids.push(j);
                j += 1;
            }

            if run_ids.len() == 1 {
                single(&ops[i]).await;
                i = j;
                continue;
            }

            let rels: Vec<&str> = run_ids.iter().map(|&id| ops[id].rel.as_str()).collect();
            let mut tree_args: Vec<String> = CFG.iter().map(|s| s.to_string()).collect();
            tree_args.extend([
                "--git-dir".to_string(), self.gitdir.to_string_lossy().to_string(),
                "--work-tree".to_string(), self.worktree.to_string_lossy().to_string(),
                "ls-tree".to_string(), "--name-only".to_string(), first.hash.clone(), "--".to_string(),
            ]);
            tree_args.extend(rels.iter().map(|r| r.to_string()));

            let tree = run_git(
                &tree_args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                &self.worktree,
            ).await;

            if !tree.ok() {
                for &id in &run_ids {
                    single(&ops[id]).await;
                }
                i = j;
                continue;
            }

            let have: std::collections::HashSet<String> = tree.text.trim()
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();

            let list: Vec<&Op> = run_ids.iter()
                .map(|&id| &ops[id])
                .filter(|op| have.contains(&op.rel))
                .collect();

            if !list.is_empty() {
                let mut co_args: Vec<String> = CFG.iter().map(|s| s.to_string()).collect();
                co_args.extend([
                    "--git-dir".to_string(), self.gitdir.to_string_lossy().to_string(),
                    "--work-tree".to_string(), self.worktree.to_string_lossy().to_string(),
                    "checkout".to_string(), first.hash.clone(), "--".to_string(),
                ]);
                co_args.extend(list.iter().map(|op| op.file.clone()));

                let co = run_git(
                    &co_args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                    &self.worktree,
                ).await;
                if !co.ok() {
                    for &id in &run_ids {
                        single(&ops[id]).await;
                    }
                    i = j;
                    continue;
                }
            }

            for &id in &run_ids {
                let op = &ops[id];
                if !have.contains(&op.rel) {
                    info!("file not in snapshot, deleting {}", op.file);
                    let _ = tokio::fs::remove_file(&op.file).await;
                }
            }

            i = j;
        }
        Ok(())
    }

    /// Unified diff text of all changes since `hash`.
    pub async fn diff(&self, hash: &str) -> Result<String> {
        let _g = self.lock.lock().await;

        self.add().await?;

        let out = self.git(&[
            QUOTE, &[
                "diff", "--cached", "--no-ext-diff", hash, "--", ".",
            ],
        ].concat()).await;

        if !out.ok() {
            warn!("snapshot diff failed (code {})", out.code);
            return Ok(String::new());
        }
        Ok(out.text.trim().to_string())
    }

    /// Full per-file diff between two tree hashes.
    pub async fn diff_full(&self, from: &str, to: &str) -> Result<Vec<FileDiff>> {
        let _g = self.lock.lock().await;

        // name-status for status classification.
        let status_out = self.git(&[
            QUOTE, &[
                "diff", "--no-ext-diff", "--name-status", "--no-renames",
                from, to, "--", ".",
            ],
        ].concat()).await;

        let mut status_map: std::collections::HashMap<String, DiffStatus> = Default::default();
        for line in status_out.text.trim().lines() {
            if line.is_empty() {
                continue;
            }
            let mut parts = line.splitn(2, '\t');
            let code = parts.next().unwrap_or("").trim();
            let file = parts.next().unwrap_or("").trim();
            if file.is_empty() {
                continue;
            }
            let s = if code.starts_with('A') { DiffStatus::Added }
                else if code.starts_with('D') { DiffStatus::Deleted }
                else { DiffStatus::Modified };
            status_map.insert(file.to_string(), s);
        }

        // numstat for line counts.
        let num_out = self.git(&[
            QUOTE, &[
                "diff", "--no-ext-diff", "--no-renames", "--numstat",
                from, to, "--", ".",
            ],
        ].concat()).await;

        struct Row {
            file: String,
            status: DiffStatus,
            binary: bool,
            additions: u32,
            deletions: u32,
        }

        let rows: Vec<Row> = num_out.text.trim()
            .lines()
            .filter(|l| !l.is_empty())
            .filter_map(|line| {
                let mut parts = line.splitn(3, '\t');
                let adds = parts.next()?.trim();
                let dels = parts.next()?.trim();
                let file = parts.next()?.trim().to_string();
                if file.is_empty() {
                    return None;
                }
                let binary = adds == "-" && dels == "-";
                let additions = if binary { 0 } else { adds.parse().unwrap_or(0) };
                let deletions = if binary { 0 } else { dels.parse().unwrap_or(0) };
                let status = status_map.remove(&file).unwrap_or(DiffStatus::Modified);
                Some(Row { file, status, binary, additions, deletions })
            })
            .collect();

        // Build result with per-file content diffs.
        let mut result: Vec<FileDiff> = Vec::with_capacity(rows.len());
        for row in &rows {
            let patch_text = if row.binary {
                String::new()
            } else {
                let (before, after) = self.file_contents(from, to, &row.file, &row.status).await;
                unified_diff(&row.file, &before, &after)
            };
            result.push(FileDiff {
                file: row.file.clone(),
                patch: patch_text,
                additions: row.additions,
                deletions: row.deletions,
                status: Some(row.status.clone()),
            });
        }

        Ok(result)
    }

    /// Run periodic garbage collection on the shadow repo.
    pub async fn cleanup(&self) -> Result<()> {
        let _g = self.lock.lock().await;
        if !self.gitdir.exists() {
            return Ok(());
        }
        let out = self.git(&["gc", "--prune=7.days.ago"]).await;
        if !out.ok() {
            warn!("snapshot gc failed (code {}): {}", out.code, out.stderr);
        } else {
            info!("snapshot gc done");
        }
        Ok(())
    }

    /// Fetch file contents at `from` (before) and `to` (after) for a single file.
    async fn file_contents(
        &self,
        from: &str,
        to: &str,
        file: &str,
        status: &DiffStatus,
    ) -> (String, String) {
        match status {
            DiffStatus::Added => {
                let after = self.git_show(to, file).await;
                (String::new(), after)
            }
            DiffStatus::Deleted => {
                let before = self.git_show(from, file).await;
                (before, String::new())
            }
            DiffStatus::Modified => {
                let before = self.git_show(from, file).await;
                let after = self.git_show(to, file).await;
                (before, after)
            }
        }
    }

    async fn git_show(&self, tree: &str, file: &str) -> String {
        let refspec = format!("{}:{}", tree, file);
        let out = self.git(&[CFG, &["show", &refspec]].concat()).await;
        if !out.ok() {
            return String::new();
        }
        out.text
    }
}

// ─── helpers ─────────────────────────────────────────────────────────────────

fn gitdir_for(worktree: &Path) -> PathBuf {
    let mut h = DefaultHasher::new();
    worktree.hash(&mut h);
    let key = format!("{:016x}", h.finish());
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("roc")
        .join("snapshot")
        .join(key)
}

async fn run_git(args: &[&str], cwd: &Path) -> Out {
    let result = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .await;

    match result {
        Ok(o) => Out {
            code: o.status.code().unwrap_or(1),
            text: String::from_utf8_lossy(&o.stdout).to_string(),
            stderr: String::from_utf8_lossy(&o.stderr).to_string(),
        },
        Err(e) => Out {
            code: 1,
            text: String::new(),
            stderr: e.to_string(),
        },
    }
}

/// Minimal unified diff generator (no external crate needed).
fn unified_diff(filename: &str, before: &str, after: &str) -> String {
    let b_lines: Vec<&str> = before.lines().collect();
    let a_lines: Vec<&str> = after.lines().collect();

    if b_lines == a_lines {
        return String::new();
    }

    let mut out = format!(
        "--- a/{}\n+++ b/{}\n",
        filename, filename
    );

    // Simple hunk: show everything (unlimited context, like the TS version).
    let start_b = 1usize;
    let start_a = 1usize;
    out.push_str(&format!(
        "@@ -{},{} +{},{} @@\n",
        start_b, b_lines.len(), start_a, a_lines.len()
    ));
    for line in &b_lines {
        out.push('-');
        out.push_str(line);
        out.push('\n');
    }
    for line in &a_lines {
        out.push('+');
        out.push_str(line);
        out.push('\n');
    }
    out
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command as StdCmd;
    use tempfile::tempdir;

    fn git_init(dir: &Path) {
        StdCmd::new("git").args(["init"]).current_dir(dir).output().unwrap();
        StdCmd::new("git").args(["config", "user.email", "t@t.com"]).current_dir(dir).output().unwrap();
        StdCmd::new("git").args(["config", "user.name", "T"]).current_dir(dir).output().unwrap();
    }

    fn git_commit(dir: &Path, name: &str, content: &str) {
        std::fs::write(dir.join(name), content).unwrap();
        StdCmd::new("git").args(["add", "."]).current_dir(dir).output().unwrap();
        StdCmd::new("git").args(["commit", "-m", "c"]).current_dir(dir).output().unwrap();
    }

    #[tokio::test]
    async fn test_track_returns_hash() {
        let dir = tempdir().unwrap();
        git_init(dir.path());
        git_commit(dir.path(), "hello.txt", "world\n");
        std::fs::write(dir.path().join("new.txt"), "content\n").unwrap();

        let mgr = SnapshotManager::for_worktree(dir.path());
        let hash = mgr.track().await.unwrap();
        assert!(hash.is_some());
        let h = hash.unwrap();
        assert_eq!(h.len(), 40);
    }

    #[tokio::test]
    async fn test_patch_lists_changed_files() {
        let dir = tempdir().unwrap();
        git_init(dir.path());
        git_commit(dir.path(), "a.txt", "before\n");

        let mgr = SnapshotManager::for_worktree(dir.path());
        let hash = mgr.track().await.unwrap().unwrap();

        std::fs::write(dir.path().join("a.txt"), "after\n").unwrap();

        let patch = mgr.patch(&hash).await.unwrap();
        assert_eq!(patch.hash, hash);
        assert_eq!(patch.files.len(), 1);
        assert!(patch.files[0].contains("a.txt"));
    }

    #[tokio::test]
    async fn test_diff_returns_text() {
        let dir = tempdir().unwrap();
        git_init(dir.path());
        git_commit(dir.path(), "f.txt", "old\n");

        let mgr = SnapshotManager::for_worktree(dir.path());
        let hash = mgr.track().await.unwrap().unwrap();

        std::fs::write(dir.path().join("f.txt"), "new\n").unwrap();

        let d = mgr.diff(&hash).await.unwrap();
        assert!(!d.is_empty());
        assert!(d.contains("-old") || d.contains("+new"));
    }

    #[tokio::test]
    async fn test_restore_reverts_changes() {
        let dir = tempdir().unwrap();
        git_init(dir.path());
        git_commit(dir.path(), "r.txt", "original\n");

        let mgr = SnapshotManager::for_worktree(dir.path());
        let hash = mgr.track().await.unwrap().unwrap();

        std::fs::write(dir.path().join("r.txt"), "modified\n").unwrap();

        mgr.restore(&hash).await.unwrap();

        let content = std::fs::read_to_string(dir.path().join("r.txt")).unwrap();
        assert_eq!(content.trim(), "original");
    }

    #[tokio::test]
    async fn test_unified_diff_basic() {
        let before = "line1\nline2\n";
        let after = "line1\nchanged\n";
        let d = unified_diff("test.txt", before, after);
        assert!(d.contains("--- a/test.txt"));
        assert!(d.contains("+++ b/test.txt"));
    }

    #[tokio::test]
    async fn test_unified_diff_identical() {
        let s = "same\n";
        assert!(unified_diff("f.txt", s, s).is_empty());
    }
}
