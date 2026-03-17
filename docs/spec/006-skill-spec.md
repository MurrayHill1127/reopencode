# Skill 模块规格文档

**版本:** 0.1.0-draft  
**创建日期:** 2026-03-17  
**状态:** 待实现  
**优先级:** MVP (核心模块)

---

## 1. 概述 (Overview)

### 模块目的

Skill 模块是 ROC (reopencode) 的可扩展指令系统核心，负责：

- **技能定义解析** - 解析 SKILL.md 文件的 YAML frontmatter 和内容
- **多源发现加载** - 从项目、用户、全局、远程 URL 四个层级发现技能
- **技能注册管理** - 管理技能的注册、查询、去重和覆盖
- **技能执行集成** - 提供工具接口，供 Agent 调用加载技能指令
- **权限控制集成** - 与权限系统集成，控制技能的可见性和访问

### 设计目标

| 目标 | 说明 | 优先级 |
|------|------|--------|
| 多源发现 | 支持项目、用户、全局、远程 URL 四个发现源 | MVP |
| YAML Frontmatter | 解析 SKILL.md 的 YAML 元数据和 Markdown 内容 | MVP |
| 优先级覆盖 | 项目级 > OpenCode 全局 > 用户 > 内置 | MVP |
| 技能去重 | 同名技能按优先级自动去重 | MVP |
| 懒加载内容 | 支持延迟加载技能内容以优化内存 | MVP |
| 权限集成 | 与权限系统集成，支持允许/拒绝模式 | MVP |
| 远程加载 | 从 URL 下载并缓存技能包 | 迭代 |
| MCP 集成 | 技能可携带嵌入式 MCP 配置 | 迭代 |
| 模板变量 | 技能内容支持变量替换 | 迭代 |

### MVP 范围 (v0.1.0)

**必须实现:**
- ✅ Skill 数据结构定义 (SkillInfo, SkillMetadata, SkillScope)
- ✅ SKILL.md 解析器 (YAML frontmatter + Markdown body)
- ✅ 多源技能发现器 (项目/用户/全局)
- ✅ SkillRegistry 注册中心
- ✅ SkillTool 工具定义
- ✅ 技能去重和优先级覆盖

**迭代实现:**
- ⏳ 远程 URL 技能下载和缓存
- ⏳ 技能模板变量解析
- ⏳ 技能嵌入式 MCP 配置
- ⏳ 技能内容懒加载
- ⏳ 内置技能定义

---

## 2. 文件索引 (File Index)

| 文件路径 | 职责 | 关键导出 | TypeScript 参考 |
|----------|------|----------|-----------------|
| `src/skill/mod.rs` | 模块根，公共 API 导出 | `Skill`, `SkillRegistry`, `SkillTool` | `skill/index.ts` |
| `src/skill/types.rs` | 数据结构定义 | `SkillInfo`, `SkillMetadata`, `SkillScope` | `skill/skill.ts:22-28` |
| `src/skill/parser.rs` | SKILL.md 解析器 | `parse_skill_file`, `SkillParseError` | `config/markdown.ts` |
| `src/skill/discovery.rs` | 技能发现器 | `discover_skills`, `DiscoveryOptions` | `skill/skill.ts:55-179` |
| `src/skill/registry.rs` | 技能注册中心 | `SkillRegistry`, `register`, `get`, `all` | `skill/skill.ts` state |
| `src/skill/loader.rs` | 远程技能加载器 | `SkillLoader`, `pull_from_url` | `skill/discovery.ts` |
| `src/skill/error.rs` | 错误类型定义 | `SkillError`, `ParseError` | `skill/skill.ts:30-46` |

### 模块结构

```
src/skill/
├── mod.rs              # 公共 API 导出 (~100 行)
├── types.rs            # 数据结构 (~150 行)
├── parser.rs           # SKILL.md 解析 (~120 行)
├── discovery.rs        # 技能发现 (~200 行)
├── registry.rs         # 注册中心 (~150 行)
├── loader.rs           # 远程加载 (~100 行)
└── error.rs            # 错误类型 (~60 行)
```

---

## 3. 数据结构 (Data Structures)

### 3.1 技能信息 (SkillInfo)

```rust
/// 技能完整信息
/// 对应 TypeScript: Skill.Info (skill/skill.ts:22-28)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillInfo {
    /// 技能名称 (必须，1-64 字符，小写字母数字和连字符)
    /// 对应 TypeScript: name: z.string()
    pub name: String,
    
    /// 技能描述 (必须，1-1024 字符)
    /// 对应 TypeScript: description: z.string()
    pub description: String,
    
    /// 技能文件路径 (绝对路径)
    /// 对应 TypeScript: location: z.string()
    pub location: std::path::PathBuf,
    
    /// 技能内容 (Markdown body，不含 frontmatter)
    /// 对应 TypeScript: content: z.string()
    pub content: String,
    
    /// 技能作用域
    pub scope: SkillScope,
    
    /// 可选元数据
    #[serde(flatten)]
    pub metadata: SkillMetadata,
}

/// 技能作用域
/// 对应 TypeScript: SkillScope (features/opencode-skill-loader/types.ts:4)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkillScope {
    /// 项目级技能 (.opencode/skills/, .claude/skills/)
    /// 优先级最高
    Project = 1,
    
    /// OpenCode 全局技能 (~/.config/opencode/skills/)
    Opencode = 2,
    
    /// 用户技能 (~/.claude/skills/, ~/.agents/skills/)
    User = 3,
    
    /// 内置技能
    /// 优先级最低
    Builtin = 4,
}

impl Default for SkillScope {
    fn default() -> Self {
        Self::Builtin
    }
}
```

### 3.2 技能元数据 (SkillMetadata)

```rust
/// 技能元数据
/// 对应 TypeScript: SkillMetadata (features/opencode-skill-loader/types.ts:6-18)
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SkillMetadata {
    /// 指定使用的模型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    
    /// 参数提示文本
    #[serde(skip_serializing_if = "Option::is_none")]
    pub argument_hint: Option<String>,
    
    /// 指定使用的 Agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    
    /// 是否为子任务
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtask: Option<bool>,
    
    /// 许可证信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    
    /// 兼容性标识
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<String>,
    
    /// 自定义元数据键值对
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
    
    /// 允许的工具列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,
}

/// 技能配置 (用于 opencode.json)
/// 对应 TypeScript: Config.Skills (config/config.ts:703-710)
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SkillConfig {
    /// 额外的技能搜索路径
    #[serde(default)]
    pub paths: Vec<String>,
    
    /// 远程技能 URL 列表
    #[serde(default)]
    pub urls: Vec<String>,
}
```

### 3.3 技能发现结果 (DiscoveryResult)

```rust
/// 技能发现结果
pub struct DiscoveryResult {
    /// 发现的所有技能
    pub skills: std::collections::HashMap<String, SkillInfo>,
    
    /// 技能目录列表
    pub directories: Vec<std::path::PathBuf>,
    
    /// 发现过程中的警告
    pub warnings: Vec<String>,
}

/// 技能发现选项
/// 对应 TypeScript: DiscoverSkillsOptions (features/opencode-skill-loader/loader.ts:38-41)
#[derive(Debug, Clone, Default)]
pub struct DiscoveryOptions {
    /// 工作目录 (默认为当前目录)
    pub directory: Option<std::path::PathBuf>,
    
    /// 是否包含 Claude Code 路径 (.claude/, .agents/)
    /// 默认为 true
    pub include_claude_paths: bool,
    
    /// 是否禁用外部技能
    pub disable_external: bool,
    
    /// 最大递归深度
    pub max_depth: usize,
}

impl Default for DiscoveryOptions {
    fn default() -> Self {
        Self {
            directory: None,
            include_claude_paths: true,
            disable_external: false,
            max_depth: 2,
        }
    }
}
```

### 3.4 技能工具输出 (SkillToolOutput)

```rust
/// 技能工具执行输出
pub struct SkillToolOutput {
    /// 输出标题
    pub title: String,
    
    /// 格式化的技能内容
    pub output: String,
    
    /// 元数据
    pub metadata: SkillToolMetadata,
}

/// 技能工具元数据
pub struct SkillToolMetadata {
    /// 技能名称
    pub name: String,
    
    /// 技能目录
    pub dir: std::path::PathBuf,
}
```

---

## 4. API 设计 (API Design)

### 4.1 公共 API (mod.rs 导出)

```rust
// src/skill/mod.rs

mod error;
mod types;
mod parser;
mod discovery;
mod registry;
mod loader;

pub use error::{SkillError, ParseError};
pub use types::{
    SkillInfo, SkillScope, SkillMetadata, SkillConfig,
    DiscoveryResult, DiscoveryOptions,
    SkillToolOutput, SkillToolMetadata,
};
pub use parser::{parse_skill_file, parse_skill_content};
pub use discovery::{discover_skills, discover_skills_with_options};
pub use registry::{SkillRegistry, SkillStore};
pub use loader::{SkillLoader, pull_skills_from_url};

/// 获取单个技能
/// 对应 TypeScript: Skill.get() (skill/skill.ts:181-183)
pub async fn get(name: &str) -> Option<SkillInfo> {
    SkillRegistry::global().get(name).await
}

/// 获取所有技能
/// 对应 TypeScript: Skill.all() (skill/skill.ts:185-187)
pub async fn all() -> Vec<SkillInfo> {
    SkillRegistry::global().all().await
}

/// 获取可用技能 (过滤权限)
/// 对应 TypeScript: Skill.available() (skill/skill.ts:193-197)
pub async fn available(permission: Option<&crate::permission::Permission>) -> Vec<SkillInfo> {
    SkillRegistry::global().available(permission).await
}

/// 获取技能目录列表
/// 对应 TypeScript: Skill.dirs() (skill/skill.ts:189-191)
pub async fn directories() -> Vec<std::path::PathBuf> {
    SkillRegistry::global().directories().await
}

/// 格式化技能列表
/// 对应 TypeScript: Skill.fmt() (skill/skill.ts:199-217)
pub fn format_skills(skills: &[SkillInfo], verbose: bool) -> String {
    if skills.is_empty() {
        return "No skills are currently available.".to_string();
    }
    
    if verbose {
        let items: Vec<String> = skills.iter().flat_map(|s| {
            vec![
                "  <skill>".to_string(),
                format!("    <name>{}</name>", s.name),
                format!("    <description>{}</description>", s.description),
                format!("    <location>{}</location>", s.location.display()),
                "  </skill>".to_string(),
            ]
        }).collect();
        
        format!("<available_skills>\n{}\n</available_skills>", items.join("\n"))
    } else {
        let items: Vec<String> = skills.iter()
            .map(|s| format!("- **{}**: {}", s.name, s.description))
            .collect();
        
        format!("## Available Skills\n{}", items.join("\n"))
    }
}
```

### 4.2 SkillRegistry

```rust
// src/skill/registry.rs

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::skill::*;

/// 技能注册中心
/// 
/// 管理所有注册的技能，提供按名称查询和权限过滤
/// 
/// # Example
/// 
/// ```rust
/// use reopencode::skill::{SkillRegistry, discover_skills};
/// 
/// #[tokio::main]
/// async fn main() {
///     let registry = SkillRegistry::global();
///     
///     // 发现并注册技能
///     let result = discover_skills(Default::default()).await;
///     registry.register_all(result.skills).await;
///     
///     // 获取技能
///     if let Some(skill) = registry.get("my-skill").await {
///         println!("Found skill: {}", skill.name);
///     }
/// }
/// ```
pub struct SkillRegistry {
    /// 技能存储 (名称 -> 技能信息)
    skills: Arc<RwLock<HashMap<String, SkillInfo>>>,
    
    /// 技能目录列表
    directories: Arc<RwLock<Vec<PathBuf>>>,
    
    /// 是否已初始化
    initialized: Arc<RwLock<bool>>,
}

impl SkillRegistry {
    /// 获取全局注册中心实例
    pub fn global() -> &'static Self {
        static INSTANCE: std::sync::OnceLock<SkillRegistry> = std::sync::OnceLock::new();
        INSTANCE.get_or_init(|| Self {
            skills: Arc::new(RwLock::new(HashMap::new())),
            directories: Arc::new(RwLock::new(Vec::new())),
            initialized: Arc::new(RwLock::new(false)),
        })
    }
    
    /// 创建新的注册中心
    pub fn new() -> Self {
        Self {
            skills: Arc::new(RwLock::new(HashMap::new())),
            directories: Arc::new(RwLock::new(Vec::new())),
            initialized: Arc::new(RwLock::new(false)),
        }
    }
    
    /// 注册单个技能
    /// 
    /// 如果同名技能已存在，按作用域优先级决定是否覆盖
    pub async fn register(&self, skill: SkillInfo) -> Result<(), SkillError> {
        let mut skills = self.skills.write().await;
        
        if let Some(existing) = skills.get(&skill.name) {
            // 按优先级决定是否覆盖 (数值越小优先级越高)
            if skill.scope < existing.scope {
                tracing::info!(
                    name = %skill.name,
                    old_scope = ?existing.scope,
                    new_scope = ?skill.scope,
                    "Overriding skill with higher priority"
                );
            } else {
                tracing::debug!(
                    name = %skill.name,
                    existing_scope = ?existing.scope,
                    "Skipping skill with lower priority"
                );
                return Ok(());
            }
        }
        
        skills.insert(skill.name.clone(), skill);
        Ok(())
    }
    
    /// 批量注册技能
    pub async fn register_all(&self, skills: HashMap<String, SkillInfo>) {
        for (_, skill) in skills {
            if let Err(e) = self.register(skill).await {
                tracing::warn!("Failed to register skill: {}", e);
            }
        }
    }
    
    /// 获取单个技能
    pub async fn get(&self, name: &str) -> Option<SkillInfo> {
        let skills = self.skills.read().await;
        skills.get(name).cloned()
    }
    
    /// 获取所有技能
    pub async fn all(&self) -> Vec<SkillInfo> {
        let skills = self.skills.read().await;
        skills.values().cloned().collect()
    }
    
    /// 获取可用技能 (过滤权限)
    pub async fn available(
        &self, 
        permission: Option<&crate::permission::Permission>,
    ) -> Vec<SkillInfo> {
        let skills = self.skills.read().await;
        
        skills.values()
            .filter(|skill| {
                if let Some(perm) = permission {
                    // 检查权限
                    !perm.is_denied("skill", &skill.name)
                } else {
                    true
                }
            })
            .cloned()
            .collect()
    }
    
    /// 获取技能目录列表
    pub async fn directories(&self) -> Vec<PathBuf> {
        let dirs = self.directories.read().await;
        dirs.clone()
    }
    
    /// 添加技能目录
    pub async fn add_directory(&self, dir: PathBuf) {
        let mut dirs = self.directories.write().await;
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    
    /// 清空注册中心
    pub async fn clear(&self) {
        let mut skills = self.skills.write().await;
        let mut dirs = self.directories.write().await;
        skills.clear();
        dirs.clear();
    }
    
    /// 初始化 (发现并加载技能)
    pub async fn initialize(&self, options: DiscoveryOptions) -> Result<(), SkillError> {
        let result = discover_skills_with_options(options).await?;
        self.register_all(result.skills).await;
        
        for dir in result.directories {
            self.add_directory(dir).await;
        }
        
        let mut initialized = self.initialized.write().await;
        *initialized = true;
        
        Ok(())
    }
    
    /// 检查是否已初始化
    pub async fn is_initialized(&self) -> bool {
        *self.initialized.read().await
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

### 4.3 技能发现器

```rust
// src/skill/discovery.rs

use std::path::{Path, PathBuf};
use crate::skill::*;
use crate::config::Config;

/// 技能发现函数
/// 
/// 从多个来源发现技能，按优先级合并
/// 
/// # 优先级 (从高到低)
/// 1. 项目级 (.opencode/skills/)
/// 2. OpenCode 全局 (~/.config/opencode/skills/)
/// 3. 用户级 (.claude/skills/, ~/.agents/skills/)
/// 4. 内置技能
/// 
/// 对应 TypeScript: discoverAllSkills() (features/opencode-skill-loader/loader.ts:43-63)
pub async fn discover_skills_with_options(options: DiscoveryOptions) -> Result<DiscoveryResult, SkillError> {
    let directory = options.directory.clone().unwrap_or_else(|| std::env::current_dir().unwrap());
    let mut result = DiscoveryResult::default();
    
    // 1. 项目级技能 (.opencode/skills/)
    let project_skills = discover_from_directory(
        &directory.join(".opencode/skills"),
        SkillScope::Project,
    ).await;
    merge_skills(&mut result.skills, project_skills, &mut result.warnings);
    
    // 2. OpenCode 全局技能
    if let Ok(opencode_dir) = crate::config::opencode_config_dir() {
        let opencode_skills = discover_from_directory(
            &opencode_dir.join("skills"),
            SkillScope::Opencode,
        ).await;
        merge_skills(&mut result.skills, opencode_skills, &mut result.warnings);
    }
    
    // 3. 用户级技能 (如果启用)
    if options.include_claude_paths {
        // .claude/skills/ (项目)
        let claude_project = discover_from_directory(
            &directory.join(".claude/skills"),
            SkillScope::User,
        ).await;
        merge_skills(&mut result.skills, claude_project, &mut result.warnings);
        
        // .agents/skills/ (项目)
        let agents_project = discover_from_directory(
            &directory.join(".agents/skills"),
            SkillScope::User,
        ).await;
        merge_skills(&mut result.skills, agents_project, &mut result.warnings);
        
        // ~/.claude/skills/ (全局)
        if let Ok(home) = std::env::var("HOME") {
            let claude_global = discover_from_directory(
                &PathBuf::from(&home).join(".claude/skills"),
                SkillScope::User,
            ).await;
            merge_skills(&mut result.skills, claude_global, &mut result.warnings);
            
            // ~/.agents/skills/ (全局)
            let agents_global = discover_from_directory(
                &PathBuf::from(&home).join(".agents/skills"),
                SkillScope::User,
            ).await;
            merge_skills(&mut result.skills, agents_global, &mut result.warnings);
        }
    }
    
    // 4. 从配置加载额外路径
    if let Ok(config) = Config::get().await {
        for path in &config.skills.paths {
            let expanded = shellexpand::tilde(&path).to_string();
            let resolved = PathBuf::from(&expanded);
            let extra_skills = discover_from_directory(&resolved, SkillScope::User).await;
            merge_skills(&mut result.skills, extra_skills, &mut result.warnings);
            result.directories.push(resolved);
        }
        
        // 5. 远程 URL 技能
        for url in &config.skills.urls {
            match pull_skills_from_url(url).await {
                Ok(skills) => {
                    merge_skills(&mut result.skills, skills, &mut result.warnings);
                }
                Err(e) => {
                    result.warnings.push(format!("Failed to load skills from {}: {}", url, e));
                }
            }
        }
    }
    
    Ok(result)
}

/// 从单个目录发现技能
async fn discover_from_directory(
    dir: &Path,
    scope: SkillScope,
) -> Vec<SkillInfo> {
    if !dir.exists() || !dir.is_dir() {
        return Vec::new();
    }
    
    let mut skills = Vec::new();
    
    // 递归查找 SKILL.md 文件
    if let Ok(entries) = walkdir::WalkDir::new(dir)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() == "SKILL.md")
        .collect::<Vec<_>>()
    {
        for entry in entries {
            let path = entry.path();
            match parse_skill_file(path, scope).await {
                Ok(skill) => skills.push(skill),
                Err(e) => {
                    tracing::warn!("Failed to parse skill {}: {}", path.display(), e);
                }
            }
        }
    }
    
    skills
}

/// 合并技能到结果中 (处理重复)
fn merge_skills(
    target: &mut HashMap<String, SkillInfo>,
    new: Vec<SkillInfo>,
    warnings: &mut Vec<String>,
) {
    for skill in new {
        if let Some(existing) = target.get(&skill.name) {
            if skill.scope >= existing.scope {
                // 已存在更高优先级的同名技能，跳过
                continue;
            }
        }
        target.insert(skill.name.clone(), skill);
    }
}

/// 便捷函数：使用默认选项发现技能
pub async fn discover_skills(directory: Option<PathBuf>) -> Result<DiscoveryResult, SkillError> {
    let options = DiscoveryOptions {
        directory,
        ..Default::default()
    };
    discover_skills_with_options(options).await
}
```

### 4.4 技能解析器

```rust
// src/skill/parser.rs

use std::path::Path;
use crate::skill::*;

/// 解析 SKILL.md 文件
/// 
/// 对应 TypeScript: ConfigMarkdown.parse() (config/markdown.ts)
pub async fn parse_skill_file(
    path: &Path,
    scope: SkillScope,
) -> Result<SkillInfo, SkillError> {
    let content = tokio::fs::read_to_string(path).await
        .map_err(|e| SkillError::IoError(path.to_path_buf(), e))?;
    
    let (frontmatter, body) = parse_frontmatter(&content)?;
    
    // 验证必须字段
    let name = frontmatter.get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SkillError::MissingField(path.to_path_buf(), "name".to_string()))?
        .to_string();
    
    let description = frontmatter.get("description")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SkillError::MissingField(path.to_path_buf(), "description".to_string()))?
        .to_string();
    
    // 解析可选元数据
    let metadata = parse_metadata(&frontmatter)?;
    
    Ok(SkillInfo {
        name,
        description,
        location: path.to_path_buf(),
        content: body.trim().to_string(),
        scope,
        metadata,
    })
}

/// 解析 YAML frontmatter
fn parse_frontmatter(content: &str) -> Result<(serde_yaml::Value, String), SkillError> {
    // 查找 frontmatter 边界
    if !content.starts_with("---\n") {
        return Ok((serde_yaml::Value::Null, content.to_string()));
    }
    
    let end = content[4..].find("\n---\n")
        .ok_or_else(|| SkillError::ParseError("Unclosed frontmatter".to_string()))?;
    
    let frontmatter_str = &content[4..end + 4];
    let body = content[end + 8..].to_string();
    
    let frontmatter: serde_yaml::Value = serde_yaml::from_str(frontmatter_str)
        .map_err(|e| SkillError::ParseError(format!("Invalid YAML: {}", e)))?;
    
    Ok((frontmatter, body))
}

/// 解析元数据
fn parse_metadata(frontmatter: &serde_yaml::Value) -> Result<SkillMetadata, SkillError> {
    let metadata = SkillMetadata {
        model: frontmatter.get("model").and_then(|v| v.as_str()).map(|s| s.to_string()),
        argument_hint: frontmatter.get("argument-hint").and_then(|v| v.as_str()).map(|s| s.to_string()),
        agent: frontmatter.get("agent").and_then(|v| v.as_str()).map(|s| s.to_string()),
        subtask: frontmatter.get("subtask").and_then(|v| v.as_bool()),
        license: frontmatter.get("license").and_then(|v| v.as_str()).map(|s| s.to_string()),
        compatibility: frontmatter.get("compatibility").and_then(|v| v.as_str()).map(|s| s.to_string()),
        metadata: frontmatter.get("metadata")
            .and_then(|v| serde_yaml::from_value(v.clone()).ok()),
        allowed_tools: frontmatter.get("allowed-tools")
            .and_then(|v| serde_yaml::from_value(v.clone()).ok()),
    };
    
    Ok(metadata)
}

/// 解析技能内容 (仅解析 frontmatter，不读取文件)
pub fn parse_skill_content(content: &str) -> Result<(String, String, SkillMetadata), SkillError> {
    let (frontmatter, body) = parse_frontmatter(content)?;
    
    let name = frontmatter.get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SkillError::ParseError("Missing 'name' field".to_string()))?
        .to_string();
    
    let description = frontmatter.get("description")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SkillError::ParseError("Missing 'description' field".to_string()))?
        .to_string();
    
    let metadata = parse_metadata(&frontmatter)?;
    
    Ok((name, description, metadata))
}
```

### 4.5 SkillTool 定义

```rust
// src/skill/tool.rs (集成到 src/tool/ 模块)

use crate::tool::*;
use crate::skill::*;

/// 技能工具定义
/// 
/// 对应 TypeScript: SkillTool (tool/skill.ts:9-104)
pub struct SkillTool;

impl SkillTool {
    /// 创建工具定义
    pub fn define() -> ToolDefinition {
        ToolDefinition {
            name: "skill".to_string(),
            description: "Load a specialized skill that provides domain-specific instructions and workflows.".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "name".to_string(),
                    description: "The name of the skill from available_skills".to_string(),
                    required: true,
                    schema: ParameterSchema::String,
                },
            ],
            execute: Box::new(|params, ctx| {
                Box::pin(async move {
                    execute_skill_tool(params, ctx).await
                })
            }),
        }
    }
}

async fn execute_skill_tool(
    params: serde_json::Value,
    ctx: ToolContext,
) -> Result<ToolOutput, ToolError> {
    let name = params["name"].as_str()
        .ok_or_else(|| ToolError::InvalidParameter("Missing 'name' parameter".to_string()))?;
    
    // 获取技能
    let skill = get(name).await
        .ok_or_else(|| {
            let available = all().await.iter()
                .map(|s| s.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            ToolError::NotFound(format!(
                "Skill '{}' not found. Available skills: {}",
                name,
                if available.is_empty() { "none".to_string() } else { available }
            ))
        })?;
    
    // 请求权限
    if let Some(perm) = &ctx.permission {
        perm.request("skill", &[name]).await?;
    }
    
    // 构建输出
    let dir = skill.location.parent().unwrap_or(std::path::Path::new(""));
    
    // 获取相关文件列表
    let files = list_skill_files(dir, 10).await;
    
    let output = format!(
        r#"<skill_content name="{}">
# Skill: {}

{}

Base directory for this skill: {}
Relative paths in this skill (e.g., scripts/, reference/) are relative to this base directory.
Note: file list is sampled.

<skill_files>
{}
</skill_files>
</skill_content>"#,
        skill.name,
        skill.name,
        skill.content,
        dir.display(),
        files.iter()
            .map(|f| format!("<file>{}</file>", f.display()))
            .collect::<Vec<_>>()
            .join("\n")
    );
    
    Ok(ToolOutput {
        title: format!("Loaded skill: {}", skill.name),
        content: output,
        metadata: Some(serde_json::json!({
            "name": skill.name,
            "dir": dir.to_string_lossy(),
        })),
    })
}

/// 列出技能目录中的文件
async fn list_skill_files(dir: &std::path::Path, limit: usize) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    
    if let Ok(mut entries) = tokio::fs::read_dir(dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.file_name() == Some(std::ffi::OsStr::new("SKILL.md")) {
                continue;
            }
            files.push(path);
            if files.len() >= limit {
                break;
            }
        }
    }
    
    files
}
```

---

## 5. 技能发现路径 (Skill Discovery Paths)

### 5.1 发现路径优先级

| 优先级 | 路径模式 | 作用域 | 示例 |
|--------|---------|--------|------|
| 1 (最高) | `<project>/.opencode/skills/**/SKILL.md` | Project | `/home/user/project/.opencode/skills/git-release/SKILL.md` |
| 2 | `~/.config/opencode/skills/**/SKILL.md` | Opencode | `/home/user/.config/opencode/skills/testing/SKILL.md` |
| 3 | `<project>/.claude/skills/**/SKILL.md` | User | `/home/user/project/.claude/skills/review/SKILL.md` |
| 4 | `<project>/.agents/skills/**/SKILL.md` | User | `/home/user/project/.agents/skills/deploy/SKILL.md` |
| 5 | `~/.claude/skills/**/SKILL.md` | User | `/home/user/.claude/skills/common/SKILL.md` |
| 6 | `~/.agents/skills/**/SKILL.md` | User | `/home/user/.agents/skills/utils/SKILL.md` |
| 7 (最低) | 内置技能 | Builtin | 代码中硬编码 |

### 5.2 SKILL.md 文件格式

```markdown
---
name: git-release
description: Create consistent releases and changelogs from git history
license: MIT
compatibility: opencode
metadata:
  audience: maintainers
  workflow: github
allowed-tools:
  - Bash
  - Read
  - Write
---

## Purpose

This skill helps you create consistent releases and changelogs from your git history.

## Workflow

1. Analyze recent commits since last tag
2. Group changes by type (feat, fix, docs, etc.)
3. Generate changelog section
4. Create release with notes

## Examples

### Creating a minor release

\`\`\`
Run: git-release --minor
\`\`\`

## Notes

- Requires git to be installed
- Works best with conventional commits
```

### 5.3 配置文件格式

```jsonc
// opencode.json
{
  "skills": {
    "paths": [
      "~/my-skills",
      "./custom-skills"
    ],
    "urls": [
      "https://example.com/skills/index.json"
    ]
  }
}
```

---

## 6. TypeScript 版本对照 (TypeScript Migration Mapping)

### 6.1 核心功能映射

| 功能 | TypeScript 位置 | Rust 目标文件 | 优先级 |
|------|-----------------|---------------|--------|
| 技能定义 | `skill/skill.ts:22-28` | `types.rs` | MVP |
| 技能发现 | `skill/skill.ts:55-179` | `discovery.rs` | MVP |
| 技能解析 | `config/markdown.ts` | `parser.rs` | MVP |
| 技能工具 | `tool/skill.ts:9-104` | `tool.rs` | MVP |
| 远程加载 | `skill/discovery.ts` | `loader.rs` | 迭代 |
| 多源合并 | `features/opencode-skill-loader/merger.ts` | `registry.rs` | MVP |

### 6.2 数据结构映射

| TypeScript Schema | Rust Struct | 字段数 | 优先级 |
|-------------------|-------------|--------|--------|
| `Skill.Info` | `SkillInfo` | 6 | MVP |
| `SkillScope` | `SkillScope` | 4 枚举值 | MVP |
| `SkillMetadata` | `SkillMetadata` | 8 | MVP |
| `Config.Skills` | `SkillConfig` | 2 | MVP |
| `DiscoveryOptions` | `DiscoveryOptions` | 4 | MVP |

### 6.3 发现路径映射

| TypeScript 路径 | Rust 路径 | 说明 |
|-----------------|-----------|------|
| `EXTERNAL_DIRS` | `discovery.rs` | `.claude`, `.agents` |
| `OPENCODE_SKILL_PATTERN` | `discovery.rs` | `.opencode/skill/` |
| `SKILL_PATTERN` | `discovery.rs` | `**/SKILL.md` |

---

## 7. 测试策略 (Test Strategy)

### 7.1 单元测试

**测试文件:** `src/skill/*.rs` (内联 `#[cfg(test)]` 模块)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;
    
    /// 测试 SKILL.md 解析
    #[tokio::test]
    async fn test_parse_skill_file() {
        let temp = TempDir::new().unwrap();
        let skill_path = temp.path().join("SKILL.md");
        
        let content = r#"---
name: test-skill
description: A test skill
---

# Test Content

This is the skill body.
"#;
        
        std::fs::write(&skill_path, content).unwrap();
        
        let skill = parse_skill_file(&skill_path, SkillScope::Project).await.unwrap();
        
        assert_eq!(skill.name, "test-skill");
        assert_eq!(skill.description, "A test skill");
        assert!(skill.content.contains("Test Content"));
        assert_eq!(skill.scope, SkillScope::Project);
    }
    
    /// 测试缺失必须字段
    #[tokio::test]
    async fn test_parse_skill_missing_name() {
        let temp = TempDir::new().unwrap();
        let skill_path = temp.path().join("SKILL.md");
        
        let content = r#"---
description: No name
---
Content
"#;
        
        std::fs::write(&skill_path, content).unwrap();
        
        let result = parse_skill_file(&skill_path, SkillScope::User).await;
        assert!(result.is_err());
    }
    
    /// 测试技能优先级覆盖
    #[tokio::test]
    async fn test_skill_priority_override() {
        let registry = SkillRegistry::new();
        
        // 先注册低优先级
        let user_skill = SkillInfo {
            name: "same-name".to_string(),
            description: "User skill".to_string(),
            location: PathBuf::from("/user/skill/SKILL.md"),
            content: "User content".to_string(),
            scope: SkillScope::User,
            metadata: SkillMetadata::default(),
        };
        registry.register(user_skill).await.unwrap();
        
        // 再注册高优先级
        let project_skill = SkillInfo {
            name: "same-name".to_string(),
            description: "Project skill".to_string(),
            location: PathBuf::from("/project/skill/SKILL.md"),
            content: "Project content".to_string(),
            scope: SkillScope::Project,
            metadata: SkillMetadata::default(),
        };
        registry.register(project_skill).await.unwrap();
        
        // 应该是项目级覆盖用户级
        let skill = registry.get("same-name").await.unwrap();
        assert_eq!(skill.description, "Project skill");
        assert_eq!(skill.scope, SkillScope::Project);
    }
    
    /// 测试技能格式化
    #[test]
    fn test_format_skills_verbose() {
        let skills = vec![
            SkillInfo {
                name: "skill-a".to_string(),
                description: "First skill".to_string(),
                location: PathBuf::from("/path/a/SKILL.md"),
                content: "".to_string(),
                scope: SkillScope::Project,
                metadata: SkillMetadata::default(),
            },
        ];
        
        let output = format_skills(&skills, true);
        assert!(output.contains("<available_skills>"));
        assert!(output.contains("<name>skill-a</name>"));
        assert!(output.contains("<description>First skill</description>"));
    }
    
    /// 测试技能格式化 (简洁模式)
    #[test]
    fn test_format_skills_simple() {
        let skills = vec![
            SkillInfo {
                name: "skill-a".to_string(),
                description: "First skill".to_string(),
                location: PathBuf::from("/path/a/SKILL.md"),
                content: "".to_string(),
                scope: SkillScope::Project,
                metadata: SkillMetadata::default(),
            },
        ];
        
        let output = format_skills(&skills, false);
        assert!(output.contains("## Available Skills"));
        assert!(output.contains("- **skill-a**: First skill"));
    }
}

### 7.2 集成测试

```rust
// tests/skill/test_integration.rs

use reopencode::skill::*;
use tempfile::TempDir;
use std::io::Write;

#[tokio::test]
async fn test_full_discovery_flow() {
    // 创建临时目录结构
    let temp = TempDir::new().unwrap();
    
    // 项目级技能
    let project_skill_dir = temp.path().join(".opencode/skills/test-skill");
    std::fs::create_dir_all(&project_skill_dir).unwrap();
    std::fs::write(
        project_skill_dir.join("SKILL.md"),
        r#"---
name: test-skill
description: Project test skill
---
Project skill content
"#,
    ).unwrap();
    
    // 发现技能
    let options = DiscoveryOptions {
        directory: Some(temp.path().to_path_buf()),
        include_claude_paths: false,
        ..Default::default()
    };
    
    let result = discover_skills_with_options(options).await.unwrap();
    
    assert!(result.skills.contains_key("test-skill"));
    let skill = result.skills.get("test-skill").unwrap();
    assert_eq!(skill.scope, SkillScope::Project);
}

#[tokio::test]
async fn test_skill_registry_integration() {
    let registry = SkillRegistry::new();
    
    // 创建测试技能
    let skill = SkillInfo {
        name: "integration-test".to_string(),
        description: "Integration test skill".to_string(),
        location: PathBuf::from("/test/SKILL.md"),
        content: "Test content".to_string(),
        scope: SkillScope::Project,
        metadata: SkillMetadata::default(),
    };
    
    // 注册
    registry.register(skill.clone()).await.unwrap();
    
    // 查询
    let retrieved = registry.get("integration-test").await;
    assert!(retrieved.is_some());
    
    // 列表
    let all = registry.all().await;
    assert!(all.iter().any(|s| s.name == "integration-test"));
}
```

### 7.3 测试覆盖要求

| 模块 | 覆盖率要求 | 关键测试场景 |
|------|-----------|--------------|
| `types.rs` | 80% | 所有 struct 序列化/反序列化 |
| `parser.rs` | 90% | YAML 解析、缺失字段、无效格式 |
| `discovery.rs` | 85% | 多源发现、优先级覆盖、目录不存在 |
| `registry.rs` | 90% | 注册、查询、去重、清空 |

---

## 附录 A: 依赖项清单

### 必须依赖 (MVP)

```toml
[dependencies]
# 异步运行时
tokio = { version = "1", features = ["full"] }

# 序列化
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"

# 错误处理
thiserror = "2"

# 文件系统
walkdir = "2"
shellexpand = "3"

# 日志
tracing = "0.1"

# 路径处理
camino = "1"
```

### 可选依赖 (Future)

```toml
[dependencies]
# HTTP 客户端 (远程技能加载)
reqwest = { version = "0.12", features = ["json"] }

# 缓存
directories = "5"
```

---

## 附录 B: 错误类型定义

```rust
// src/skill/error.rs

/// 技能错误
#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("IO 错误 '{0}': {1}")]
    IoError(std::path::PathBuf, #[source] std::io::Error),
    
    #[error("解析错误: {0}")]
    ParseError(String),
    
    #[error("缺失字段 '{1}' in {0}")]
    MissingField(std::path::PathBuf, String),
    
    #[error("技能未找到: {0}")]
    NotFound(String),
    
    #[error("技能已存在: {0}")]
    AlreadyExists(String),
    
    #[error("无效的技能名称: {0}")]
    InvalidName(String),
    
    #[error("远程加载失败: {0}")]
    RemoteLoadError(String),
    
    #[error("权限被拒绝: {0}")]
    PermissionDenied(String),
}

/// 解析错误
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("无效的 YAML: {0}")]
    InvalidYaml(String),
    
    #[error("未闭合的 frontmatter")]
    UnclosedFrontmatter,
    
    #[error("缺失必须字段: {0}")]
    MissingRequiredField(String),
    
    #[error("无效的字段值: {0}")]
    InvalidFieldValue(String),
}
```

---

## 附录 C: 实现检查清单

### Phase 1: 基础架构

- [ ] 创建所有模块文件框架
- [ ] 定义错误类型 (`error.rs`)
- [ ] 实现基础类型 (`types.rs`)
- [ ] 实现 YAML frontmatter 解析 (`parser.rs`)
- [ ] 添加单元测试框架

### Phase 2: 核心功能

- [ ] 实现技能发现器 (`discovery.rs`)
- [ ] 实现技能注册中心 (`registry.rs`)
- [ ] 实现多源发现逻辑
- [ ] 实现优先级覆盖
- [ ] 实现技能去重

### Phase 3: 工具集成

- [ ] 实现 SkillTool 定义
- [ ] 集成到工具注册中心
- [ ] 集成权限系统
- [ ] 集成系统提示

### Phase 4: 远程加载 (迭代)

- [ ] 实现远程 URL 技能下载
- [ ] 实现技能缓存机制
- [ ] 实现增量更新

### Phase 5: 测试与文档

- [ ] 完成所有单元测试
- [ ] 完成集成测试
- [ ] 添加使用文档
- [ ] 添加示例技能

---

**文档维护:** 此文档应随代码实现同步更新，所有 API 变更需在此文档中反映。