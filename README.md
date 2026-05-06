# ReOpenCode

[opencode](https://github.com/anomalyco/opencode) 的 Rust 重写版本——相同的功能集，单一二进制，无运行时依赖。

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)

---

## 为什么要重写

原版 opencode 基于 TypeScript，意味着需要 Node.js、臃肿的 `node_modules` 以及不可忽视的启动开销。本项目将其直接移植到 Rust：一个可以随处运行的独立二进制文件。

目标：
- 在关键路径上与 opencode 行为对齐
- 快速启动，低内存占用
- 除复制二进制文件外无需任何安装步骤

这不是 fork——TypeScript 源码仅作为参考规范，而非依赖。

---

## 当前状态

核心功能已可用。服务器、会话管理、Provider 集成和大部分工具均已实现并通过测试，TUI 正在持续完善中。

**已完成：**
- HTTP 服务器，29 个接口（会话 CRUD、流式输出、工具执行、Provider 鉴权）
- SQLite 持久化的会话管理
- SSE 流式 LLM 响应
- Provider 支持：Anthropic、OpenAI、Azure、Google、OpenRouter、xAI、Vertex、智谱 GLM
- 工具实现：edit、bash、read、write、glob、grep、web fetch、web search、apply\_patch 等 17+ 个
- TUI（ratatui）：对话界面、会话列表、可折叠侧边栏、文件树、命令面板、主题系统
- 斜杠命令：`/exit`、`/new`、`/clear`、`/help`、`/sessions`
- MCP 客户端（stdio + HTTP 传输）
- 多层级配置合并与环境变量替换
- Category、Command、Hook、Skill 子系统

---

## 技术栈

| 领域 | Crate |
|------|-------|
| 异步运行时 | tokio |
| HTTP 服务器 | axum + tower |
| HTTP 客户端 | reqwest |
| TUI | ratatui + crossterm |
| 数据库 | sqlx (SQLite) |
| CLI | clap |
| MCP 协议 | rmcp |
| 序列化 | serde + serde\_json |
| 流式处理 | async-stream + futures |

---

## 构建

需要 Rust 1.75+。

```sh
# 调试构建
cargo build

# 发布构建
cargo build --release

# 运行
./target/release/reopencode
```

启动服务器：

```sh
./target/release/reopencode serve
```

启动 TUI（会自动连接 `http://127.0.0.1:4096`）：

```sh
./target/release/reopencode run
```

---

## TUI 快捷键

| 按键 | 功能 |
|------|------|
| `Enter` | 发送消息 |
| `Ctrl+J` / `Shift+Enter` | 输入框内换行 |
| `Ctrl+B` | 展开 / 折叠左侧会话侧边栏 |
| `Ctrl+R` | 展开 / 折叠右侧信息侧边栏（上下文、MCP、LSP、Diffs） |
| `Ctrl+P` | 打开会话列表浮层 |
| `Ctrl+M` | 查看 MCP 状态 |
| `Ctrl+C` | 取消流式输出 / 退出 |
| `Tab` | 在输入框和消息列表间切换焦点 |
| `j` / `k` / `↑` / `↓` | 滚动消息（消息列表聚焦时） |
| `g` / `G` | 跳到顶部 / 底部 |

**斜杠命令**（在输入框中输入）：

| 命令 | 功能 |
|------|------|
| `/exit` 或 `/quit` | 退出 |
| `/new [标题]` | 新建会话 |
| `/clear` | 清空当前对话 |
| `/sessions` | 切换侧边栏 |
| `/help` | 显示帮助信息 |

---

## 开发

```sh
# 运行测试
cargo test --lib

# 代码检查
cargo clippy -- -D warnings

# 格式化
cargo fmt

# 保存时自动重新编译
cargo watch -x run
```

运行单个测试：

```sh
cargo test <测试名称>
```

---

## 目录结构

```
src/
├── main.rs          入口
├── lib.rs           库根
├── agent/           Agent 循环与注册
├── provider/        LLM Provider 适配器
├── session/         会话生命周期、消息存储、流式处理
├── server/          axum HTTP 服务器及路由处理器
├── tool/            工具实现（17+ 个）
├── mcp/             MCP 客户端
├── pty/             伪终端会话
├── config/          配置加载与合并
├── storage/         存储后端（SQLite）
├── bus/             内部事件总线
├── category/        分类系统
├── command/         命令处理
├── hook/            生命周期钩子
├── skill/           Skill 加载器
└── cli/             CLI 命令与 TUI
    └── commands/tui/    ratatui 界面与组件
```

---

## 提交规范

```
feat: 新增 X
fix: 修复 Y 边界情况
refactor: 简化 Z
test: 覆盖 W
docs: 更新文档
```

---

## License

MIT — 详见 [LICENSE](LICENSE)。
