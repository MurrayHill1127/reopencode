# ReOpenCode (ROC)

> opencode 的 Rust 重写版本 —— 更快、更安全、零运行时依赖

## 为什么重写

原 opencode 使用 TypeScript 编写，存在：
- 运行时依赖复杂（Node.js + 一堆 npm 包）
- 类型系统不够严格
- 启动慢、内存占用高
- 分发需要带整个 node_modules

ROC 用 Rust 重写，目标：
- **编译时类型安全** - 错误在编译期就暴露
- **零运行时依赖** - 单个二进制文件，直接跑
- **性能提升** - 启动快、内存占用低
- **更好的插件系统** - 从设计初期就考虑可扩展性

## 技术栈

- **语言:** Rust (latest stable)
- **参考项目:**
  - [opencode](https://github.com/anomalyco/opencode) - 核心功能
  - [oh-my-opencode](https://github.com/code-yeongyu/oh-my-openagent) - 插件系统设计

## 项目结构

```
~/ROC/
├── reopencode/          # ROC 主项目 (Rust)
├── opencode/            # 参考实现 (TS)
└── oh-my-openagent/     # 插件系统参考
```

## 开发进度

- [x] 项目初始化
- [ ] 核心架构设计
- [ ] 基础功能实现
- [ ] 插件系统
- [ ] 第一个可用版本

## 贡献

项目处于早期开发阶段。如果你感兴趣，欢迎提 issue 或 PR。

## License

MIT

---

*ReOpenCode Team · 2026*
