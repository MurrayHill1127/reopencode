# ReOpenCode

基于 Rust 的 AI 编程助手，提供完整的 Agent 工具执行能力、终端 TUI 界面和 HTTP REST API。

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)
[![Tests](https://img.shields.io/badge/tests-1888%20passed-brightgreen.svg)]()

---

<p align="center">
  <a href="https://www.atlascloud.ai/?utm_source=github&utm_medium=link&utm_campaign=reopencode">
    <img src="./assets/atlas-cloud-logo.png" alt="Atlas Cloud" width="200">
  </a>
</p>

> 🎁 **[Atlas Cloud](https://www.atlascloud.ai/?utm_source=github&utm_medium=link&utm_campaign=reopencode)** 提供兼容 OpenAI 格式的统一 LLM API（支持 Claude、DeepSeek、Qwen 等 59+ 模型），可作为 ReOpenCode 的 LLM 提供商之一直接接入，并通过 Media API 为编程助手扩展图像/视频生成等多模态能力 — [coding plan](https://www.atlascloud.ai/console/coding-plan)

<details>
<summary>All Atlas Cloud LLM models (59)</summary>

- Anthropic: `anthropic/claude-haiku-4.5-20251001`, `anthropic/claude-opus-4.8`, `anthropic/claude-sonnet-4.6`
- OpenAI: `openai/gpt-5.4`, `openai/gpt-5.5`
- Google Gemini: `google/gemini-3.1-flash-lite`, `google/gemini-3.1-pro-preview`, `google/gemini-3.5-flash`
- Qwen: `qwen/qwen2.5-7b-instruct`, `Qwen/Qwen3-235B-A22B-Instruct-2507`, `qwen/qwen3-235b-a22b-thinking-2507`, `qwen/qwen3-30b-a3b`, `Qwen/Qwen3-30B-A3B-Instruct-2507`, `qwen/qwen3-30b-a3b-thinking-2507`, `qwen/qwen3-32b`, `qwen/qwen3-8b`, `Qwen/Qwen3-Coder`, `qwen/qwen3-coder-next`, `qwen/qwen3-max-2026-01-23`, `Qwen/Qwen3-Next-80B-A3B-Instruct`, `Qwen/Qwen3-Next-80B-A3B-Thinking`, `Qwen/Qwen3-VL-235B-A22B-Instruct`, `qwen/qwen3-vl-235b-a22b-thinking`, `qwen/qwen3-vl-30b-a3b-instruct`, `qwen/qwen3-vl-30b-a3b-thinking`, `qwen/qwen3-vl-8b-instruct`, `qwen/qwen3.5-122b-a10b`, `qwen/qwen3.5-27b`, `qwen/qwen3.5-35b-a3b`, `qwen/qwen3.5-397b-a17b`, `qwen/qwen3.6-35b-a3b`, `qwen/qwen3.6-plus`
- DeepSeek: `deepseek-ai/deepseek-ocr`, `deepseek-ai/deepseek-r1-0528`, `deepseek-ai/DeepSeek-V3-0324`, `deepseek-ai/DeepSeek-V3.1`, `deepseek-ai/DeepSeek-V3.1-Terminus`, `deepseek-ai/deepseek-v3.2`, `deepseek-ai/DeepSeek-V3.2-Exp`, `deepseek-ai/deepseek-v4-flash`, `deepseek-ai/deepseek-v4-pro`
- Kimi: `moonshotai/Kimi-K2-Instruct`, `moonshotai/Kimi-K2-Instruct-0905`, `moonshotai/Kimi-K2-Thinking`, `moonshotai/kimi-k2.5`, `moonshotai/kimi-k2.6`
- GLM: `zai-org/GLM-4.6`, `zai-org/glm-4.7`, `zai-org/glm-5`, `zai-org/glm-5-turbo`, `zai-org/glm-5.1`, `zai-org/glm-5v-turbo`
- MiniMax: `MiniMaxAI/MiniMax-M2`, `minimaxai/minimax-m2.1`, `minimaxai/minimax-m2.5`, `minimaxai/minimax-m2.7`
- xAI: `xai/grok-4.3`
- KAT: `kwaipilot/kat-coder-pro-v2`
- Other: `owl`

</details>

---

## 项目特点

- **纯 Rust 实现**：单一二进制文件，无运行时依赖，不依赖 Node.js/npm
- **完整 Agent 循环**：LLM 可调用 18 个工具，结果自动回传，支持最多 20 轮多步推理
- **双向接口**：终端 TUI（ratatui）和 HTTP REST API（axum），均可独立使用
- **安全设计**：危险工具执行前通过权限系统检查，工具执行前后自动打 shadow git 快照
- **高性能**：Rust 编译型语言的零成本抽象和内存安全保证

## 快速开始

### 安装依赖

- Rust 1.75+
- macOS / Linux / Windows

### 编译

```bash
git clone https://github.com/MurrayHill1127/reopencode.git
cd reopencode
cargo build --release
```

### 配置 API Key

在 `~/.config/roc/roc.toml` 中创建配置文件：

```toml
[server]
port = 4096
host = "127.0.0.1"

[provider.kimi]
api-key = "sk-xxxxxxxxxxxxxxxxxx"
```

或者设置环境变量：

```bash
export KIMI_API_KEY="sk-xxxxxxxxxxxxxxxxxx"
# 也支持 OPENAI_API_KEY / ANTHROPIC_API_KEY
```

### 启动

```bash
# 启动服务器（后台）
./target/release/reopencode serve &

# 启动 TUI 终端界面
./target/release/reopencode run
```

TUI 启动后会自动连接 `http://127.0.0.1:4096`。在输入框中输入消息，按 Enter 发送，即可与 AI 对话。

---

## Agent 工具列表

| 工具 | 功能 | 说明 |
|------|------|------|
| `bash` | Shell 命令执行 | 30 秒超时，自动重试 2 次 |
| `read` | 读取文件 | 支持文本文件读取 |
| `write` | 写入文件 | 支持新文件创建和覆盖 |
| `edit` | 精确编辑 | 字符串替换，支持 replaceAll |
| `apply_patch` | 应用补丁 | 标准 unified diff 格式 |
| `multiedit` | 多文件编辑 | 批量修改多个文件 |
| `glob` | Glob 搜索 | 通配符文件匹配 |
| `grep` | 代码搜索 | 基于 ripgrep 的高性能搜索 |
| `ls` | 列出目录 | 文件和目录列表 |
| `codesearch` | 语义搜索 | 代码库内容搜索 |
| `webfetch` | 网页抓取 | URL 内容获取 |
| `websearch` | 网页搜索 | 在线搜索 |
| `todo_write` | 任务列表写入 | 创建和管理结构化任务 |
| `todo_read` | 任务列表读取 | 读取当前任务状态 |
| `task` | Agent 任务 | 启动子 Agent 执行任务 |
| `question` | 用户提问 | 挂起等待用户回答 |
| `lsp` | LSP 操作 | hover/definition/references |
| `mcp_*` | MCP 工具代理 | 自动注册外部 MCP 服务工具 |

## TUI 界面

### 快捷键

| 按键 | 功能 |
|------|------|
| `Enter` | 发送消息 |
| `Ctrl+J` / `Shift+Enter` | 输入框内换行 |
| `Ctrl+B` | 左侧会话侧边栏（会话列表 + Token 用量） |
| `Ctrl+R` | 右侧信息侧边栏（上下文/MCP/LSP/Diffs） |
| `Alt+E` | 文件树浏览器 |
| `Alt+K` | 命令面板（新建会话/列表/MCP/模型/帮助） |
| `Ctrl+\`` | 代码块折叠/展开 |
| `Ctrl+P` | 会话列表浮层 |
| `Ctrl+M` | MCP 服务器状态 |
| `Ctrl+C` | 取消流式输出 / 退出 |
| `Esc` | 第 1 次：取消流式；第 2 次：清空输入框 |
| `↑` / `↓`（空输入） | 提示历史导航 |
| `Ctrl+←` / `Ctrl+→` | 按单词跳转 |
| `Ctrl+W` / `Alt+Backspace` | 删除前一个单词 |

### 斜杠命令

在输入框中输入以 `/` 开头的命令：

| 命令 | 功能 |
|------|------|
| `/exit` | 退出程序 |
| `/new [标题]` | 创建新会话 |
| `/clear` | 清空当前对话 |
| `/sessions` | 切换会话侧边栏 |
| `/undo` / `/redo` | 撤销/恢复上一条消息 |
| `/copy` | 复制最后一条 AI 回复到剪贴板 |
| `/compact` | 触发 AI 会话压缩（上下文过长时） |
| `/debug` | 显示最后一条消息的调试信息 |
| `/help` | 显示帮助信息 |

### Shell 模式

输入以 `!` 开头的消息，直接在本机执行命令并将结果显示在对话中。

```
!ls -la
!cargo test --lib
```

### 界面预览

```
┌─ Header ───────────────────────────────────────────────────────────┐
│ roc  Session Title                            ░░░░░░░░░░░  0%      │
├────────────────────────────────────────────────────────────────────┤
│ ┌─ Sidebar ──┬─ Chat Area ────────────────────┬─ Info Panel ──┐  │
│ │ Sessions   │                                 │ Context       │  │
│ │ ────────   │  you                            │ ────────      │  │
│ │ ● Current  │    hi                           │ Tokens: 0     │  │
│ │   Older    │                                 │ Usage: 0%     │  │
│ │            │  build · claude  5.0s           │               │  │
│ │ ────────   │    Hello! How can I help...    │ MCP Servers   │  │
│ │ build      │                                 │ ────────      │  │
│ │ ░░░░ 0%   │                                 │               │  │
│ │            │                                 │ LSP Servers   │  │
│ │            │                                 │ ────────      │  │
│ └────────────┴─────────────────────────────────┴───────────────┘  │
├─ Footer ───────────────────────────────────────────────────────────┤
│ ~/Projects/ROC/reopencode                     build·cl-3.5  I  0%  │
└────────────────────────────────────────────────────────────────────┘
```

## HTTP API

### 会话管理

| 方法 | 路径 | 功能 |
|------|------|------|
| `GET` | `/session` | 列出所有会话 |
| `POST` | `/session` | 创建新会话 |
| `GET` | `/session/{id}` | 获取会话详情 |
| `PATCH` | `/session/{id}` | 更新会话（标题/归档） |
| `DELETE` | `/session/{id}` | 删除会话 |

### 消息处理

| 方法 | 路径 | 功能 |
|------|------|------|
| `POST` | `/session/{id}/message` | 同步发送消息（含完整 Agent 循环） |
| `POST` | `/session/{id}/stream` | 流式发送消息（SSE） |
| `POST` | `/session/{id}/prompt_async` | 异步后台处理（立即返回 204） |
| `POST` | `/session/{id}/abort` | 取消当前处理 |

### 会话操作

| 方法 | 路径 | 功能 |
|------|------|------|
| `POST` | `/session/{id}/fork` | 从指定消息分叉新会话 |
| `POST` | `/session/{id}/revert` | 回滚会话到指定消息 |
| `POST` | `/session/{id}/unrevert` | 取消回滚状态 |
| `POST` | `/session/{id}/undo` | 撤销最后一条消息对 |
| `POST` | `/session/{id}/redo` | 恢复被撤销的消息 |
| `POST` | `/session/{id}/summarize` | AI 压缩会话上下文 |
| `POST` | `/session/{id}/init` | 初始化项目上下文（AGENTS.md 生成） |

### 权限管理

| 方法 | 路径 | 功能 |
|------|------|------|
| `GET` | `/permission` | 列出待处理的权限请求 |
| `POST` | `/permission/{id}/reply` | 回复权限请求（once/always/reject） |
| `POST` | `/session/{id}/permissions/{id}` | 会话级权限回复 |

### 工具集成

| 方法 | 路径 | 功能 |
|------|------|------|
| `GET` | `/mcp` | 列出 MCP 服务器状态 |
| `POST` | `/mcp` | 添加 MCP 服务器 |
| `POST` | `/mcp/{name}/connect` | 连接 MCP 服务器 |
| `POST` | `/mcp/{name}/disconnect` | 断开 MCP 服务器 |
| `GET` | `/lsp` | 列出 LSP 连接状态 |
| `GET` | `/worktree` | 列出工作区 |
| `POST` | `/worktree` | 创建隔离工作区 |
| `DELETE` | `/worktree/{name}` | 删除工作区 |

### Provider 认证

| 方法 | 路径 | 功能 |
|------|------|------|
| `PUT` | `/global/auth/{provider}` | 设置 Provider 认证信息 |
| `DELETE` | `/global/auth/{provider}` | 删除 Provider 认证信息 |
| `GET` | `/provider` | 列出支持的服务商 |

### 配置与工具

| 方法 | 路径 | 功能 |
|------|------|------|
| `GET` | `/global/health` | 健康检查 |
| `GET` | `/global/config` | 获取全局配置 |
| `PATCH` | `/global/config` | 更新全局配置 |
| `GET` | `/config` | 获取当前配置 |
| `GET` | `/command` | 列出可用命令 |
| `GET` | `/skill` | 列出可用技能 |

---

## Provider 支持

| 服务商 | 文本生成 | 工具调用 | 流式输出 |
|--------|----------|----------|----------|
| OpenAI（兼容 Kimi/Azure 等） | ✅ | ✅ | ✅ |
| Anthropic | ✅ | ✅ | ✅ |
| Google / Vertex | ✅ | 取决于兼容性 | ✅ |
| xAI / Mistral / Groq | ✅ | 取决于兼容性 | ✅ |
| OpenRouter | ✅ | ✅ | ✅ |
| 智谱 GLM | ✅ | 取决于兼容性 | ✅ |

支持 14 个 Provider，共 2000+ 种模型。

---

## 架构概览

```
src/
├── main.rs                程序入口
├── lib.rs                 库根
├── agent/                 Agent 循环、配置、提示词管理
├── provider/              LLM Provider 适配器（14 个）
├── session/               会话生命周期、消息存储（SQLite）
├── server/                axum HTTP 服务器（29 个端点）
├── tool/                  18 个工具实现
├── mcp/                   MCP 协议客户端（stdio + HTTP）
├── lsp/                   LSP JSON-RPC 客户端
├── permission/            权限规则评估、ask/reply
├── snapshot/              Shadow git 快照管理
├── worktree/              Git worktree 隔离
├── config/                多层配置加载（TOML + 环境变量）
├── storage/               SQLite/JSON 后端
├── bus/                   内部事件总线
├── hook/                  生命周期钩子系统
├── skill/                 技能发现与加载
├── command/               斜杠命令系统
└── cli/commands/tui/      ratatui 终端界面（15 个组件）
```

---

## 测试

```bash
# 运行所有测试
cargo test --lib

# 运行集成测试（需要真实的 API Key）
KIMI_API_KEY="sk-xxx" cargo test --test kimi_integration

# 代码检查
cargo clippy -- -D warnings

# 格式化检查
cargo fmt -- --check
```

测试统计：

| 类别 | 数量 | 状态 |
|------|------|------|
| 单元测试 | 1888 | ✅ 全部通过 |
| 集成测试 | 3 | ✅ 全部通过 |
| 忽略的测试 | 1 | ⏸ 多字节字符处理（TODO） |

---

## 开发指南

### 添加新工具

1. 在 `src/tool/` 下创建新文件
2. 实现 `Tool` trait（`name()`、`description()`、`parameters()`、`execute()`）
3. 在 `src/server/mod.rs` 的 `build_tool_registry()` 中注册
4. 在 `src/tool/mod.rs` 中添加 `pub mod`

```rust
use async_trait::async_trait;
use crate::tool::traits::{Tool, ToolResult};
use crate::tool::error::Result;

pub struct MyTool;

#[async_trait]
impl Tool for MyTool {
    fn name(&self) -> &str { "my_tool" }
    fn description(&self) -> &str { "Description of my tool" }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        Ok(ToolResult::new("done"))
    }
}
```

### 添加新 Provider

1. 在 `src/provider/` 下创建新文件
2. 实现 `Provider` trait
3. 在 `src/provider/mod.rs` 中导出

---

## License

MIT
