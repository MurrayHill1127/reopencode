# ReOpenCode

一个用 Rust 编写的 AI 编程助手，提供完整的 agent 工具执行能力、终端 TUI 界面和 HTTP API。

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)

---

## 功能特性

### Agent 能力
- **完整工具循环**：LLM 可调用 17+ 个工具，结果自动回传，最多 20 轮迭代
- **文件操作**：read / write / edit / apply_patch / multiedit
- **代码搜索**：glob / grep / ls / codesearch
- **Shell 执行**：bash（30 秒超时，自动重试）
- **网络访问**：webfetch / websearch
- **任务管理**：todo / task / question（支持挂起等待用户回答）
- **LSP 集成**：hover / definition / references（需本地 language server）
- **MCP 工具代理**：连接外部 MCP 服务器，工具自动注册到 LLM 可见列表

### 安全与可靠性
- **权限系统**：危险工具（bash/edit/write）执行前通过 PermissionStore 检查
- **快照/还原**：工具执行前后自动打 shadow git 快照，支持文件级回滚
- **并发隔离**：同一 session 同时只允许一个请求处理
- **工具错误重试**：失败自动重试最多 2 次

### TUI 界面
- **实时流式输出**：SSE 流式渲染，3 字符微块动画
- **Markdown 渲染**：代码块盒子边框、标题分色、引用块、任务列表、链接
- **工具调用可视化**：◆ edit / ▶ bash / ▷ read / ⌕ grep 各有专属样式，diff 着色
- **Reasoning 块**：暖琥珀色背景 + `╎` 虚线左边框，可折叠
- **双侧边栏**：左侧会话列表（Ctrl+B），右侧信息面板（Ctrl+R）
- **文件树**：Alt+E 切换，j/k 导航
- **命令面板**：Alt+K，6 个命令可执行
- **斜杠命令**：/exit /new /clear /help /sessions /undo /redo /copy /compact /debug
- **提示历史**：↑/↓ 回显历史消息
- **Shell 模式**：`!cmd` 直接执行本地命令
- **代码折叠**：Ctrl+\` 折叠/展开代码块
- **Tokyo Night 配色**：深蓝灰背景，Ghostty 终端适配

### HTTP API
- **会话管理**：CRUD + fork / revert / unrevert / summarize / undo / redo
- **流式消息**：`POST /session/{id}/stream`（SSE）
- **同步消息**：`POST /session/{id}/message`（含完整 agent loop）
- **后台执行**：`POST /session/{id}/prompt_async`（立即返回 204）
- **权限回复**：`POST /session/{id}/permissions/{id}`
- **Worktree**：GET/POST/DELETE `/worktree`
- **MCP 管理**：连接/断开/状态查询
- **认证管理**：PUT/DELETE `/global/auth/{provider}`

### Provider 支持
| Provider | 文本 | 工具调用 |
|----------|------|----------|
| OpenAI / Kimi（OpenAI 兼容） | ✅ | ✅ |
| Anthropic | ✅ | ✅ |
| Azure / Google / Vertex / xAI / OpenRouter / 智谱 | ✅ | 取决于兼容性 |

---

## 快速开始

### 依赖

- Rust 1.75+
- SQLite（通过 sqlx 内嵌）

### 构建

```bash
git clone https://github.com/MurrayHill1127/reopencode.git
cd reopencode
cargo build --release
```

### 配置

在 `~/.config/roc/roc.toml` 中配置 API key（或直接设置环境变量）：

```toml
[provider.kimi]
api-key = "your-api-key"
```

或者：

```bash
export KIMI_API_KEY="your-api-key"
# 也支持 OPENAI_API_KEY / ANTHROPIC_API_KEY
```

### 启动

```bash
# 启动服务器（后台）
./target/release/reopencode serve &

# 启动 TUI
./target/release/reopencode run
```

---

## TUI 快捷键

| 按键 | 功能 |
|------|------|
| `Enter` | 发送消息 |
| `Ctrl+J` / `Shift+Enter` | 输入框内换行 |
| `Ctrl+B` | 左侧会话侧边栏 |
| `Ctrl+R` | 右侧信息侧边栏（Token、MCP、LSP） |
| `Alt+E` | 文件树 |
| `Alt+K` | 命令面板 |
| `Ctrl+\`` | 代码块折叠/展开 |
| `Ctrl+P` | 会话列表浮层 |
| `Ctrl+M` | MCP 状态 |
| `Ctrl+C` | 取消流式 / 退出 |
| `Esc` | 第 1 次取消流式，第 2 次清空输入框 |
| `↑` / `↓`（空输入） | 提示历史导航 |
| `Ctrl+Left/Right` | 按单词跳转 |
| `Ctrl+W` | 删除前一个单词 |
| `Tab` | 焦点切换 / 斜杠命令补全 |

**斜杠命令**（输入框中输入）：

| 命令 | 功能 |
|------|------|
| `/exit` | 退出 |
| `/new [标题]` | 新建会话 |
| `/clear` | 清空对话 |
| `/sessions` | 切换侧边栏 |
| `/undo` / `/redo` | 撤销/恢复消息 |
| `/copy` | 复制最后一条回复到剪贴板 |
| `/compact` | 触发 AI 会话压缩 |
| `/debug` | 显示最后一条消息的调试信息 |
| `/help` | 显示帮助 |

**Shell 模式**：输入 `!cmd` 直接在本地执行 shell 命令，结果显示在对话中。

---

## 项目结构

```
src/
├── agent/          Agent 循环与注册
├── provider/       LLM Provider 适配器（OpenAI/Anthropic/Azure/Google 等）
├── session/        会话生命周期、消息存储、流式处理
├── server/         axum HTTP 服务器及路由处理器（29 个端点）
├── tool/           工具实现（17+ 个）
├── mcp/            MCP 客户端（stdio + HTTP）
├── lsp/            LSP 客户端（JSON-RPC）
├── snapshot/       Shadow git 快照/还原
├── permission/     权限规则评估与 ask/reply
├── worktree/       Git worktree 管理
├── config/         多层级配置加载（TOML + 环境变量）
├── storage/        SQLite 持久化
└── cli/
    └── commands/tui/   ratatui TUI 界面与组件
```

---

## 开发

```bash
# 运行测试
cargo test --lib

# 代码检查
cargo clippy -- -D warnings

# 格式化
cargo fmt
```

---

## License

MIT
