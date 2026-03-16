//! Configuration data structures
//!
//! Defines all config types with serde support for TOML parsing.

use std::collections::{BTreeMap, HashMap};
use serde::{Deserialize, Serialize};

// Import CategoryConfig from category module
use crate::category::CategoryConfig;

// ==================== Root Config ====================

/// ROC 根配置结构
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// JSON Schema URL (可选，用于 IDE 提示)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// HTTP 服务器配置
    #[serde(default)]
    pub server: ServerConfig,

    /// Agent 配置集合
    #[serde(default)]
    pub agent: AgentConfigs,

    /// 提供商配置 Map
    #[serde(default)]
    pub provider: HashMap<String, ProviderConfig>,

    /// MCP 服务器配置 Map
    #[serde(default)]
    pub mcp: HashMap<String, McpConfig>,

    /// 权限规则
    #[serde(default)]
    pub permission: PermissionConfig,

    // Future fields - keep as default for forward compatibility
    #[serde(default)]
    pub command: HashMap<String, CommandConfig>,
    
    #[serde(default)]
    pub skills: SkillsConfig,
    
    #[serde(default)]
    pub storage: StorageConfig,
    
    #[serde(default)]
    pub hook: HookConfig,
    
    #[serde(default)]
    pub category: CategoryConfig,

    /// 其他未定义字段 (保留扩展性)
    #[serde(flatten)]
    pub extra: BTreeMap<String, toml::Value>,
}

// ==================== Server Config ====================

/// HTTP 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ServerConfig {
    /// 监听端口 (默认: 4096)
    #[serde(default = "default_server_port")]
    pub port: u16,

    /// 监听地址 (默认: "127.0.0.1")
    #[serde(default = "default_server_host")]
    pub host: String,

    /// 启用 mDNS 广播 (默认: false)
    #[serde(default)]
    pub mdns: bool,

    /// CORS 允许的来源
    #[serde(default)]
    pub cors_origin: Vec<String>,
}

fn default_server_port() -> u16 { 4096 }
fn default_server_host() -> String { "127.0.0.1".to_string() }

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: default_server_port(),
            host: default_server_host(),
            mdns: false,
            cors_origin: Vec::new(),
        }
    }
}

// ==================== Agent Config ====================

/// Agent 配置集合
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AgentConfigs {
    /// 命名 Agent 配置
    #[serde(flatten)]
    pub agents: BTreeMap<String, AgentConfig>,
}

/// 单个 Agent 配置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AgentConfig {
    /// 模型标识 (格式: "provider/model") (MVP)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// 温度参数 (0.0-2.0, 默认: 0.1)
    #[serde(default = "default_temperature")]
    pub temperature: f64,

    /// Top-p 采样
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    /// 系统提示词
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    /// 描述信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// 所属分类
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    /// 启用的 Skills
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<String>>,

    /// 禁用的工具列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable: Option<Vec<String>>,

    /// 运行模式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<AgentMode>,

    /// 主题色
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,

    /// 权限配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<PermissionConfig>,
}

fn default_temperature() -> f64 { 0.1 }

/// Agent 运行模式枚举
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentMode {
    Interactive,
    #[default]
    Primary,
    Subagent,
}

// ==================== Provider Config ====================

/// 提供商配置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ProviderConfig {
    /// API 密钥 (支持 {env:VAR} 语法)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// API 端点 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_url: Option<String>,

    /// 启用的模型列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<String>>,

    /// 白名单模型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whitelist: Option<Vec<String>>,

    /// 黑名单模型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blacklist: Option<Vec<String>>,
}

// ==================== MCP Config ====================

/// MCP 服务器配置 (本地或远程)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpConfig {
    /// 本地 MCP 服务器
    Local(McpLocalConfig),
    /// 远程 MCP 服务器
    Remote(McpRemoteConfig),
}

/// 本地 MCP 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct McpLocalConfig {
    /// 命令 (如: "node", "python")
    pub command: String,

    /// 命令参数
    #[serde(default)]
    pub args: Vec<String>,

    /// 环境变量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,

    /// 工作目录
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

/// 远程 MCP 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct McpRemoteConfig {
    /// 服务器 URL
    pub url: String,

    /// 认证 Token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,

    /// 超时时间 (毫秒)
    #[serde(default = "default_mcp_timeout")]
    pub timeout: u64,
}

fn default_mcp_timeout() -> u64 { 30000 }

impl Default for McpRemoteConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            token: None,
            timeout: default_mcp_timeout(),
        }
    }
}

// ==================== Permission Config ====================

/// 权限规则配置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PermissionConfig {
    /// 默认策略
    #[serde(default)]
    pub default: PermissionPolicy,

    /// 按工具分类的权限规则
    #[serde(default)]
    pub rules: HashMap<String, PermissionRule>,
}

/// 权限策略枚举
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionPolicy {
    Allow,
    Deny,
    #[default]
    Ask,
}

/// 单个权限规则
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PermissionRule {
    /// 策略
    pub policy: PermissionPolicy,

    /// 匹配的正则表达式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,

    /// 描述信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// ==================== Future Types (stubs) ====================

/// 自定义命令配置 (Future)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CommandConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Skills 配置 (Future)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SkillsConfig {
    #[serde(default)]
    pub sources: Vec<String>,
}

/// 存储配置 (Future)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct StorageConfig {
    #[serde(default)]
    pub r#type: StorageType,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageType {
    #[default]
    Sqlite,
    File,
    Memory,
}

/// Hook 配置 (Future)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookConfig {}