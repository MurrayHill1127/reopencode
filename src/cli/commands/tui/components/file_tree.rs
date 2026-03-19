//! File Tree Component - hierarchical file browser with git status
//!
//! Provides a tree view of files and directories with:
//! - Expand/collapse for directories
//! - Git status indicators (modified, added, deleted, untracked)
//! - Keyboard navigation
//! - File selection

use super::{Component, ComponentId, EventPropagation};
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List as TuiList, ListItem, ListState},
};
use std::{
    collections::HashMap,
    fmt::Display,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

// ==================== Git Status ====================

/// Git file status for tracking changes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GitFileStatus {
    /// No changes or not in git
    #[default]
    Unmodified,
    /// Modified working tree file
    Modified,
    /// New file (staged or unstaged)
    Added,
    /// Deleted file
    Deleted,
    /// Renamed file
    Renamed,
    /// Untracked file
    Untracked,
}

impl GitFileStatus {
    /// Get the display character for this status
    pub fn to_char(&self) -> char {
        match self {
            GitFileStatus::Unmodified => ' ',
            GitFileStatus::Modified => 'M',
            GitFileStatus::Added => 'A',
            GitFileStatus::Deleted => 'D',
            GitFileStatus::Renamed => 'R',
            GitFileStatus::Untracked => '?',
        }
    }

    /// Get the color for this status
    pub fn to_color(&self) -> Color {
        match self {
            GitFileStatus::Unmodified => Color::Reset,
            GitFileStatus::Modified => Color::Yellow,
            GitFileStatus::Added => Color::Green,
            GitFileStatus::Deleted => Color::Red,
            GitFileStatus::Renamed => Color::Magenta,
            GitFileStatus::Untracked => Color::Cyan,
        }
    }

    /// Check if this represents a change
    pub fn is_changed(&self) -> bool {
        !matches!(self, GitFileStatus::Unmodified)
    }
}

impl Display for GitFileStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_char())
    }
}

// ==================== File Node ====================

/// A node in the file tree (file or directory)
#[derive(Debug, Clone)]
pub struct FileNode {
    /// Display name (file or directory name)
    pub name: String,
    /// Full path relative to root
    pub path: PathBuf,
    /// Whether this is a directory
    pub is_dir: bool,
    /// Git status (None = not in git or unmodified)
    pub git_status: GitFileStatus,
    /// Child nodes (empty for files)
    pub children: Vec<FileNode>,
    /// Whether directory is expanded
    pub expanded: bool,
    /// Depth in tree (0 = root)
    pub depth: usize,
    /// Whether this node is ignored by git
    pub ignored: bool,
}

impl FileNode {
    /// Create a new file node
    pub fn new(name: String, path: PathBuf, is_dir: bool) -> Self {
        Self {
            name,
            path,
            is_dir,
            git_status: GitFileStatus::default(),
            children: Vec::new(),
            expanded: false,
            depth: 0,
            ignored: false,
        }
    }

    /// Create a file node
    pub fn file(name: String, path: PathBuf) -> Self {
        Self::new(name, path, false)
    }

    /// Create a directory node
    pub fn dir(name: String, path: PathBuf) -> Self {
        Self::new(name, path, true)
    }

    /// Set git status
    pub fn with_git_status(mut self, status: GitFileStatus) -> Self {
        self.git_status = status;
        self
    }

    /// Set depth
    pub fn with_depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    /// Set expanded state
    pub fn with_expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// Check if this node has children
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Check if this is an expandable directory
    pub fn is_expandable(&self) -> bool {
        self.is_dir && self.has_children()
    }

    /// Get icon for this node
    pub fn icon(&self) -> &str {
        if self.is_dir {
            if self.expanded { "▼" } else { "▶" }
        } else {
            " "
        }
    }

    /// Get file type icon
    pub fn file_icon(&self) -> &str {
        if self.is_dir { "📁" } else { "📄" }
    }
}

impl Display for FileNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let indent = "  ".repeat(self.depth);
        let icon = self.icon();
        let file_icon = self.file_icon();
        let git = if self.git_status.is_changed() {
            format!(" [{}]", self.git_status.to_char())
        } else {
            String::new()
        };
        write!(f, "{}{} {}{}{}", indent, icon, file_icon, self.name, git)
    }
}

// ==================== File Tree State ====================

/// State for the file tree component
#[derive(Debug, Clone, Default)]
pub struct FileTreeState {
    /// Root nodes
    pub roots: Vec<FileNode>,
    /// Flattened visible nodes for rendering
    pub visible: Vec<FileNode>,
    /// Currently selected index
    pub selected: usize,
    /// Show hidden files
    pub show_hidden: bool,
    /// Scroll offset for rendering
    pub scroll_offset: usize,
}

impl FileTreeState {
    /// Create new empty state
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuild the visible nodes list from roots
    pub fn rebuild_visible(&mut self) {
        self.visible.clear();
        self.flatten_nodes(&self.roots.clone());
    }

    /// Recursively flatten nodes
    fn flatten_nodes(&mut self, nodes: &[FileNode]) {
        for node in nodes {
            // Skip hidden files if not showing
            if !self.show_hidden && node.name.starts_with('.') {
                continue;
            }
            self.visible.push(node.clone());
            if node.is_dir && node.expanded {
                self.flatten_nodes(&node.children);
            }
        }
    }

    /// Get the currently selected node
    pub fn selected_node(&self) -> Option<&FileNode> {
        self.visible.get(self.selected)
    }

    /// Get mutable reference to selected node's source in roots
    pub fn selected_node_mut(&mut self) -> Option<&mut FileNode> {
        let selected_path = self.visible.get(self.selected)?.path.clone();
        Self::find_node_mut(&selected_path, &mut self.roots)
    }

    /// Find a node by path recursively (standalone function to avoid borrow issues)
    fn find_node_mut<'a>(path: &Path, nodes: &'a mut [FileNode]) -> Option<&'a mut FileNode> {
        for node in nodes.iter_mut() {
            if node.path == path {
                return Some(node);
            }
            if path.starts_with(&node.path)
                && let Some(found) = Self::find_node_mut(path, &mut node.children)
            {
                return Some(found);
            }
        }
        None
    }

    /// Move selection up
    pub fn prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down
    pub fn next(&mut self) {
        if self.selected < self.visible.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    /// Move to first item
    pub fn first(&mut self) {
        self.selected = 0;
    }

    /// Move to last item
    pub fn last(&mut self) {
        if !self.visible.is_empty() {
            self.selected = self.visible.len() - 1;
        }
    }

    /// Page up
    pub fn page_up(&mut self, page_size: usize) {
        self.selected = self.selected.saturating_sub(page_size);
    }

    /// Page down
    pub fn page_down(&mut self, page_size: usize) {
        self.selected = (self.selected + page_size).min(self.visible.len().saturating_sub(1));
    }

    /// Toggle expand/collapse of selected directory
    pub fn toggle_selected(&mut self) -> bool {
        let selected_path = match self.visible.get(self.selected) {
            Some(n) => n.path.clone(),
            None => return false,
        };

        if let Some(node) = Self::find_node_mut(&selected_path, &mut self.roots)
            && node.is_dir
            && node.has_children()
        {
            node.expanded = !node.expanded;
            self.rebuild_visible();
            return true;
        }
        false
    }

    /// Expand selected directory
    pub fn expand_selected(&mut self) -> bool {
        let selected_path = match self.visible.get(self.selected) {
            Some(n) => n.path.clone(),
            None => return false,
        };

        if let Some(node) = Self::find_node_mut(&selected_path, &mut self.roots)
            && node.is_dir
            && !node.expanded
            && node.has_children()
        {
            node.expanded = true;
            self.rebuild_visible();
            return true;
        }
        false
    }

    /// Collapse selected directory
    pub fn collapse_selected(&mut self) -> bool {
        let selected_path = match self.visible.get(self.selected) {
            Some(n) => n.path.clone(),
            None => return false,
        };

        if let Some(node) = Self::find_node_mut(&selected_path, &mut self.roots)
            && node.is_dir
            && node.expanded
        {
            node.expanded = false;
            self.rebuild_visible();
            return true;
        }
        false
    }

    /// Collapse all directories
    pub fn collapse_all(&mut self) {
        Self::collapse_nodes(&mut self.roots);
        self.rebuild_visible();
    }

    fn collapse_nodes(nodes: &mut [FileNode]) {
        for node in nodes.iter_mut() {
            node.expanded = false;
            Self::collapse_nodes(&mut node.children);
        }
    }

    /// Check if tree is empty
    pub fn is_empty(&self) -> bool {
        self.visible.is_empty()
    }

    /// Get visible count
    pub fn len(&self) -> usize {
        self.visible.len()
    }
}

// ==================== File Tree Component ====================

/// File tree component for TUI
pub struct FileTree {
    id: ComponentId,
    title: String,
    root_path: PathBuf,
    state: FileTreeState,
    focused: bool,
    git_status_cache: HashMap<PathBuf, GitFileStatus>,
    is_git_repo: bool,
}

impl FileTree {
    /// Create a new file tree with the given root path
    pub fn new(root_path: PathBuf) -> Self {
        let is_git_repo = Self::check_git_repo(&root_path);
        Self {
            id: ComponentId::new(),
            title: "Files".to_string(),
            root_path,
            state: FileTreeState::new(),
            focused: false,
            git_status_cache: HashMap::new(),
            is_git_repo,
        }
    }

    /// Create with a title
    pub fn with_title(root_path: PathBuf, title: impl Into<String>) -> Self {
        let mut this = Self::new(root_path);
        this.title = title.into();
        this
    }

    /// Check if directory is a git repository
    fn check_git_repo(path: &Path) -> bool {
        Command::new("git")
            .args(["rev-parse", "--is-inside-work-tree"])
            .current_dir(path)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Set the root nodes
    pub fn set_roots(&mut self, roots: Vec<FileNode>) {
        self.state.roots = roots;
        self.state.rebuild_visible();
    }

    /// Load files from the root directory
    pub fn load_from_directory(&mut self) -> std::io::Result<()> {
        let nodes = self.read_directory(&self.root_path, 0)?;
        self.state.roots = nodes;

        if self.is_git_repo {
            self.refresh_git_status();
        }

        self.state.rebuild_visible();
        Ok(())
    }

    /// Read directory contents recursively
    fn read_directory(&self, dir: &Path, depth: usize) -> std::io::Result<Vec<FileNode>> {
        let mut nodes = Vec::new();
        let entries = std::fs::read_dir(dir)?;

        for entry in entries {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            let relative = path
                .strip_prefix(&self.root_path)
                .unwrap_or(&path)
                .to_path_buf();
            let is_dir = entry.file_type()?.is_dir();

            // Skip .git directory
            if name == ".git" {
                continue;
            }

            let mut node = if is_dir {
                FileNode::dir(name, relative)
            } else {
                FileNode::file(name, relative)
            };
            node.depth = depth;

            // Recursively read children for directories
            if is_dir {
                node.children = self.read_directory(&path, depth + 1)?;
            }

            nodes.push(node);
        }

        // Sort: directories first, then alphabetically
        nodes.sort_by(|a, b| {
            if a.is_dir != b.is_dir {
                b.is_dir.cmp(&a.is_dir)
            } else {
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            }
        });

        Ok(nodes)
    }

    /// Refresh git status
    pub fn refresh_git_status(&mut self) {
        if !self.is_git_repo {
            return;
        }

        self.git_status_cache.clear();

        let output = match Command::new("git")
            .args(["status", "--porcelain", "-uall"])
            .current_dir(&self.root_path)
            .output()
        {
            Ok(o) => o,
            Err(_) => return,
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.len() < 3 {
                continue;
            }

            let x = line.chars().next().unwrap_or(' ');
            let y = line.chars().nth(1).unwrap_or(' ');
            let path_str = &line[3..];

            // Determine status from XY codes
            let status = Self::parse_git_status_xy(x, y);
            let path = PathBuf::from(path_str);

            self.git_status_cache.insert(path.clone(), status);

            // For renamed files, the path might be "old -> new"
            if path_str.contains(" -> ")
                && let Some(old_path) = path_str.split(" -> ").next()
            {
                self.git_status_cache
                    .insert(PathBuf::from(old_path), status);
            }
        }

        // Apply git status to nodes
        let cache = &self.git_status_cache;
        Self::apply_git_status_to_nodes(cache, &mut self.state.roots);
        self.state.rebuild_visible();
    }

    /// Parse git status XY codes
    fn parse_git_status_xy(x: char, y: char) -> GitFileStatus {
        match (x, y) {
            ('?', '?') => GitFileStatus::Untracked,
            ('A', _) | (_, 'A') => GitFileStatus::Added,
            ('D', _) | (_, 'D') => GitFileStatus::Deleted,
            ('R', _) | (_, 'R') => GitFileStatus::Renamed,
            ('M', _) => GitFileStatus::Modified,
            (' ', 'M') => GitFileStatus::Modified,
            _ => GitFileStatus::Unmodified,
        }
    }

    /// Apply git status to nodes recursively
    fn apply_git_status_to_nodes(
        cache: &std::collections::HashMap<PathBuf, GitFileStatus>,
        nodes: &mut [FileNode],
    ) {
        for node in nodes.iter_mut() {
            if let Some(&status) = cache.get(&node.path) {
                node.git_status = status;
            }

            Self::apply_git_status_to_nodes(cache, &mut node.children);
        }
    }

    /// Get title
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set title
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    /// Get root path
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    /// Set root path
    pub fn set_root_path(&mut self, path: PathBuf) {
        self.root_path = path;
        self.is_git_repo = Self::check_git_repo(&self.root_path);
    }

    /// Get selected file path
    pub fn selected_path(&self) -> Option<PathBuf> {
        self.state
            .selected_node()
            .map(|n| self.root_path.join(&n.path))
    }

    /// Get selected node
    pub fn selected_node(&self) -> Option<&FileNode> {
        self.state.selected_node()
    }

    /// Toggle hidden files visibility
    pub fn toggle_hidden(&mut self) {
        self.state.show_hidden = !self.state.show_hidden;
        self.state.rebuild_visible();
    }

    /// Check if showing hidden files
    pub fn show_hidden(&self) -> bool {
        self.state.show_hidden
    }

    /// Check if is git repo
    pub fn is_git_repo(&self) -> bool {
        self.is_git_repo
    }

    /// Collapse all directories
    pub fn collapse_all(&mut self) {
        self.state.collapse_all();
    }

    /// Refresh the file tree
    pub fn refresh(&mut self) -> std::io::Result<()> {
        self.load_from_directory()
    }
}

impl Component for FileTree {
    fn id(&self) -> ComponentId {
        self.id
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        // Build block with title
        let block = if self.focused {
            Block::default()
                .title(self.title.as_str())
                .borders(Borders::ALL)
                .border_style(
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
        } else {
            Block::default()
                .title(self.title.as_str())
                .borders(Borders::ALL)
        };

        // Build list items
        let items: Vec<ListItem> = self
            .state
            .visible
            .iter()
            .enumerate()
            .map(|(i, node)| {
                let indent = "  ".repeat(node.depth);
                let expand_icon = node.icon();
                let file_icon = node.file_icon();

                // Build spans for the line
                let mut spans = vec![
                    Span::raw(indent),
                    Span::styled(expand_icon, Style::default().fg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::styled(
                        file_icon,
                        Style::default().fg(if node.is_dir {
                            Color::Blue
                        } else {
                            Color::Reset
                        }),
                    ),
                    Span::raw(" "),
                    Span::raw(&node.name),
                ];

                // Add git status indicator if changed
                if node.git_status.is_changed() {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(
                        format!("[{}]", node.git_status.to_char()),
                        Style::default().fg(node.git_status.to_color()),
                    ));
                }

                let line = Line::from(spans);

                // Apply selection highlight
                if i == self.state.selected {
                    ListItem::new(line).style(
                        Style::default()
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    ListItem::new(line)
                }
            })
            .collect();

        let list = TuiList::new(items).block(block).highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(Some(self.state.selected));

        frame.render_stateful_widget(list, area, &mut state);
    }

    fn handle_input(&mut self, event: KeyEvent) -> EventPropagation {
        use crossterm::event::{KeyCode, KeyModifiers};

        if !self.focused {
            return EventPropagation::Continue;
        }

        match (event.code, event.modifiers) {
            // Navigation
            (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                self.state.prev();
                EventPropagation::Stop
            }
            (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::CONTROL) => {
                self.state.next();
                EventPropagation::Stop
            }
            (KeyCode::Home, _) | (KeyCode::Char('g'), _) => {
                self.state.first();
                EventPropagation::Stop
            }
            (KeyCode::End, _) => {
                self.state.last();
                EventPropagation::Stop
            }
            (KeyCode::PageUp, _) => {
                self.state.page_up(10);
                EventPropagation::Stop
            }
            (KeyCode::PageDown, _) => {
                self.state.page_down(10);
                EventPropagation::Stop
            }

            // Expand/collapse
            (KeyCode::Right, _) | (KeyCode::Char('l'), _) => {
                self.state.expand_selected();
                EventPropagation::Stop
            }
            (KeyCode::Left, _) | (KeyCode::Char('h'), _) => {
                self.state.collapse_selected();
                EventPropagation::Stop
            }
            (KeyCode::Enter, _) | (KeyCode::Char(' '), _) => {
                self.state.toggle_selected();
                EventPropagation::Stop
            }

            // Toggle hidden files
            (KeyCode::Char('.'), _) => {
                self.toggle_hidden();
                EventPropagation::Stop
            }

            // Refresh
            (KeyCode::Char('r'), _) => {
                let _ = self.refresh();
                EventPropagation::Stop
            }

            _ => EventPropagation::Continue,
        }
    }

    fn update(&mut self, _delta: Duration) {}

    fn is_focusable(&self) -> bool {
        true
    }

    fn focused(&self) -> bool {
        self.focused
    }

    fn on_focus(&mut self) {
        self.focused = true;
    }

    fn on_blur(&mut self) {
        self.focused = false;
    }
}

impl Default for FileTree {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_file_status_to_char() {
        assert_eq!(GitFileStatus::Unmodified.to_char(), ' ');
        assert_eq!(GitFileStatus::Modified.to_char(), 'M');
        assert_eq!(GitFileStatus::Added.to_char(), 'A');
        assert_eq!(GitFileStatus::Deleted.to_char(), 'D');
        assert_eq!(GitFileStatus::Renamed.to_char(), 'R');
        assert_eq!(GitFileStatus::Untracked.to_char(), '?');
    }

    #[test]
    fn test_git_file_status_to_color() {
        assert_eq!(GitFileStatus::Modified.to_color(), Color::Yellow);
        assert_eq!(GitFileStatus::Added.to_color(), Color::Green);
        assert_eq!(GitFileStatus::Deleted.to_color(), Color::Red);
        assert_eq!(GitFileStatus::Untracked.to_color(), Color::Cyan);
    }

    #[test]
    fn test_git_file_status_is_changed() {
        assert!(!GitFileStatus::Unmodified.is_changed());
        assert!(GitFileStatus::Modified.is_changed());
        assert!(GitFileStatus::Added.is_changed());
        assert!(GitFileStatus::Deleted.is_changed());
        assert!(GitFileStatus::Renamed.is_changed());
        assert!(GitFileStatus::Untracked.is_changed());
    }

    #[test]
    fn test_file_node_creation() {
        let file = FileNode::file("test.rs".to_string(), PathBuf::from("test.rs"));
        assert!(!file.is_dir);
        assert_eq!(file.name, "test.rs");
        assert!(!file.has_children());

        let dir = FileNode::dir("src".to_string(), PathBuf::from("src"));
        assert!(dir.is_dir);
        assert_eq!(dir.name, "src");
    }

    #[test]
    fn test_file_node_with_modifiers() {
        let node = FileNode::file("test.rs".to_string(), PathBuf::from("test.rs"))
            .with_git_status(GitFileStatus::Modified)
            .with_depth(2);

        assert_eq!(node.git_status, GitFileStatus::Modified);
        assert_eq!(node.depth, 2);
    }

    #[test]
    fn test_file_node_icon() {
        let file = FileNode::file("test.rs".to_string(), PathBuf::from("test.rs"));
        assert_eq!(file.icon(), " ");
        assert_eq!(file.file_icon(), "📄");

        let mut dir = FileNode::dir("src".to_string(), PathBuf::from("src"));
        assert_eq!(dir.icon(), "▶");
        assert_eq!(dir.file_icon(), "📁");

        dir.expanded = true;
        assert_eq!(dir.icon(), "▼");
    }

    #[test]
    fn test_file_tree_state_navigation() {
        let mut state = FileTreeState::new();

        // Create test nodes
        state.roots = vec![
            FileNode::dir("src".to_string(), PathBuf::from("src")),
            FileNode::file("main.rs".to_string(), PathBuf::from("main.rs")),
        ];
        state.rebuild_visible();

        assert_eq!(state.len(), 2);
        assert_eq!(state.selected, 0);

        state.next();
        assert_eq!(state.selected, 1);

        state.next(); // Should stay at last
        assert_eq!(state.selected, 1);

        state.prev();
        assert_eq!(state.selected, 0);

        state.prev(); // Should stay at first
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn test_file_tree_state_first_last() {
        let mut state = FileTreeState::new();
        state.roots = vec![
            FileNode::file("a".to_string(), PathBuf::from("a")),
            FileNode::file("b".to_string(), PathBuf::from("b")),
            FileNode::file("c".to_string(), PathBuf::from("c")),
        ];
        state.rebuild_visible();

        state.last();
        assert_eq!(state.selected, 2);

        state.first();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn test_file_tree_state_expand_collapse() {
        let mut state = FileTreeState::new();

        let mut src_dir = FileNode::dir("src".to_string(), PathBuf::from("src"));
        src_dir.expanded = false;
        src_dir.children = vec![FileNode::file(
            "main.rs".to_string(),
            PathBuf::from("src/main.rs"),
        )];

        state.roots = vec![src_dir];
        state.rebuild_visible();

        // Only the collapsed directory should be visible
        assert_eq!(state.len(), 1);

        // Expand
        state.expand_selected();
        state.rebuild_visible();
        assert_eq!(state.len(), 2); // src + main.rs

        // Collapse
        state.collapse_selected();
        state.rebuild_visible();
        assert_eq!(state.len(), 1);
    }

    #[test]
    fn test_file_tree_component_id() {
        let t1 = FileTree::new(PathBuf::from("."));
        let t2 = FileTree::new(PathBuf::from("."));
        assert_ne!(t1.id(), t2.id());
    }

    #[test]
    fn test_file_tree_is_focusable() {
        let tree = FileTree::new(PathBuf::from("."));
        assert!(tree.is_focusable());
    }

    #[test]
    fn test_file_tree_focus_state() {
        let mut tree = FileTree::new(PathBuf::from("."));
        assert!(!tree.focused());

        tree.on_focus();
        assert!(tree.focused());

        tree.on_blur();
        assert!(!tree.focused());
    }

    #[test]
    fn test_parse_git_status_xy() {
        assert_eq!(
            FileTree::parse_git_status_xy('?', '?'),
            GitFileStatus::Untracked
        );
        assert_eq!(
            FileTree::parse_git_status_xy('A', ' '),
            GitFileStatus::Added
        );
        assert_eq!(
            FileTree::parse_git_status_xy(' ', 'A'),
            GitFileStatus::Added
        );
        assert_eq!(
            FileTree::parse_git_status_xy('D', ' '),
            GitFileStatus::Deleted
        );
        assert_eq!(
            FileTree::parse_git_status_xy('R', ' '),
            GitFileStatus::Renamed
        );
        assert_eq!(
            FileTree::parse_git_status_xy('M', ' '),
            GitFileStatus::Modified
        );
        assert_eq!(
            FileTree::parse_git_status_xy(' ', 'M'),
            GitFileStatus::Modified
        );
        assert_eq!(
            FileTree::parse_git_status_xy(' ', ' '),
            GitFileStatus::Unmodified
        );
    }

    #[test]
    fn test_file_tree_toggle_hidden() {
        let mut tree = FileTree::new(PathBuf::from("."));
        assert!(!tree.show_hidden());

        tree.toggle_hidden();
        assert!(tree.show_hidden());

        tree.toggle_hidden();
        assert!(!tree.show_hidden());
    }
}
