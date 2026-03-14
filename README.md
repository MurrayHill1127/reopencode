# ReOpenCode (ROC)

> opencode + oh-my-opencode 的 Rust 重写版本 —— 更快、更安全、零运行时依赖

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.94.0+-orange.svg)](https://www.rust-lang.org)
[![Status](https://img.shields.io/badge/status-early--dev-yellow.svg)]()

---

## 为什么重写

原 opencode 使用 TypeScript 编写，存在：
- ❌ 运行时依赖复杂（Node.js + 一堆 npm 包）
- ❌ 类型系统不够严格
- ❌ 启动慢、内存占用高
- ❌ 分发需要带整个 node_modules

ROC 用 Rust 重写，目标：
- ✅ **编译时类型安全** - 错误在编译期就暴露
- ✅ **零运行时依赖** - 单个二进制文件，直接跑
- ✅ **性能提升** - 启动快、内存占用低
- ✅ **更好的插件系统** - 从设计初期就考虑可扩展性

---

## 技术栈

| 领域 | 技术 | 理由 |
|------|------|------|
| **语言** | Rust 1.94+ | 编译时安全、零运行时依赖 |
| **异步运行时** | tokio | Rust 最成熟的异步运行时 |
| **CLI 框架** | clap | Rust 标准 CLI 库 |
| **TUI 框架** | ratatui | Rust 最成熟的 TUI 框架 |
| **HTTP 框架** | axum | 轻量、tower 生态 |
| **数据库** | SQLite + sqlx | 轻量、类型安全 |
| **LSP** | tower-lsp | 基于 tower，与 axum 兼容 |
| **AST 分析** | tree-sitter | 多语言支持、性能好 |

---

## 参考项目

- **[opencode](https://github.com/anomalyco/opencode)** - 核心功能重写参考
- **[oh-my-opencode](https://github.com/code-yeongyu/oh-my-openagent)** - 插件系统设计参考

---

## 项目结构

```
.
├── src/
│   ├── main.rs              # CLI 入口
│   ├── lib.rs               # 库入口
│   ├── cli/                 # CLI 模块
│   ├── agent/               # Agent 系统
│   ├── tool/                # Tool 系统
│   ├── provider/            # AI Provider
│   ├── session/             # Session 管理
│   └── ...
├── Cargo.toml               # 项目配置
├── README.md                # 项目说明
└── .gitignore               # Git 忽略规则
```

---

## 开发进度

### Phase 1: MVP 核心 ✅ (Day 1)
- [x] 项目初始化
- [x] Agent trait 定义
- [x] Tool trait 定义
- [x] CLI 入口（clap）
- [x] TUI MVP（ratatui）
- [x] 4 个单元测试通过

### Phase 2: 核心功能 🔄 (Day 2-7)
- [ ] Provider 系统（OpenAI API 集成）
- [ ] 完整 Tool 系统（26 个工具）
- [ ] Session 管理（持久化存储）
- [ ] TUI 改进（文件树、语法高亮、主题）

### Phase 3: TUI 实现 (Day 8-14)
- [ ] 基础布局（侧边栏 + 主内容区）
- [ ] 文件树浏览
- [ ] 对话界面（用户输入 + AI 回复）
- [ ] 工具执行结果显示
- [ ] 键盘快捷键
- [ ] 主题/配色方案

### Phase 4: 插件系统 (Day 15-28)
- [ ] Hook 系统
- [ ] Category 系统
- [ ] Skill 系统
- [ ] Command 系统

### Phase 5: 完善与测试 (Day 29-50)
- [ ] 性能优化
- [ ] 完整测试套件
- [ ] 文档完善

---

## 快速开始

### 环境要求

- Rust 1.94+
- Cargo

### 编译

```bash
# Debug 模式
cargo build

# Release 模式（推荐）
cargo build --release
```

### 运行

```bash
# 运行 CLI
cargo run --release

# 运行 TUI
cargo run --release -- run

# 运行测试
cargo test

# 运行所有测试（包括集成测试）
cargo test --all
```

---

## 开发规范

### Commit 规范

```
<类型>: <简短描述>

[详细描述（可选）]
```

**类型:**
- `feat`: 新功能
- `fix`: Bug 修复
- `docs`: 文档更新
- `style`: 代码格式
- `refactor`: 重构
- `test`: 测试
- `chore`: 构建/工具

**示例:**
```
feat: add OpenAI provider implementation

- Implement Provider trait for OpenAI
- Add API key management
- Add error handling for rate limits
```

### 测试规范

- 每个核心模块必须包含单元测试
- 测试覆盖率目标：>80%
- 提交前确保 `cargo test` 通过
- 提交前确保 `cargo build --release` 通过

---

## 贡献

项目处于早期开发阶段。如果你感兴趣，欢迎：

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

---

## License

MIT License - 详见 [LICENSE](LICENSE) 文件

---

## 相关链接

- [GitHub 仓库](https://github.com/MurrayHill1127/reopencode)
- [opencode 官网](https://opencode.ai)
- [oh-my-opencode](https://github.com/code-yeongyu/oh-my-openagent)
- [Rust 语言](https://www.rust-lang.org)

---

*ReOpenCode Team · 2026*

*最后更新：2026-03-14*
