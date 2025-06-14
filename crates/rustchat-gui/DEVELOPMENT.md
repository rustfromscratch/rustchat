# RustChat GUI 开发指南

## 项目概述

RustChat GUI 是基于 Tauri + Svelte + TypeScript 的桌面客户端，提供现代化的聊天界面和本地性能优化。

## 技术栈

- **前端**: Svelte 5 + TypeScript + Vite
- **后端**: Rust + Tauri 2.0
- **构建工具**: Vite (前端) + Cargo (后端)
- **包管理**: npm (前端依赖) + Cargo (Rust 依赖)

## 开发环境设置

### 前置要求

1. **Node.js** (推荐 v18+)
2. **Rust** (推荐最新稳定版)
3. **Tauri CLI** (通过 npm 安装的本地版本)

### 启动开发环境

```bash
# 进入 GUI 项目目录
cd crates/rustchat-gui

# 安装前端依赖 (如果还没安装)
npm install

# 启动开发环境 (热重载)
npm run tauri dev
```

开发环境启动后：
- **前端开发服务器**: http://localhost:1420/
- **桌面应用窗口**: 自动打开
- **热重载**: 前端和后端代码变更都会自动重载

### 可用脚本

```bash
# 前端开发
npm run dev          # 仅启动前端开发服务器
npm run build        # 构建前端生产版本
npm run preview      # 预览前端构建结果

# 类型检查
npm run check        # 运行 TypeScript 和 Svelte 类型检查
npm run check:watch  # 监听模式下的类型检查

# Tauri 命令
npm run tauri dev    # 启动完整开发环境
npm run tauri build  # 构建生产版本桌面应用
```

## 项目结构

```
crates/rustchat-gui/
├── src/                    # Svelte 前端源码
│   ├── lib/               # 组件库
│   ├── routes/            # 页面路由
│   └── app.html           # HTML 模板
├── src-tauri/             # Rust 后端源码
│   ├── src/
│   │   ├── main.rs        # Tauri 应用入口
│   │   └── lib.rs         # 库代码
│   ├── Cargo.toml         # Rust 依赖配置
│   └── tauri.conf.json    # Tauri 应用配置
├── static/                # 静态资源
├── package.json           # 前端依赖和脚本
├── vite.config.js         # Vite 配置
├── svelte.config.js       # Svelte 配置
└── tsconfig.json          # TypeScript 配置
```

## 开发流程

### 1. 前端开发
- 在 `src/` 目录下开发 Svelte 组件和页面
- 使用 TypeScript 确保类型安全
- Vite 提供快速热重载

### 2. 后端开发
- 在 `src-tauri/src/` 目录下开发 Rust 代码
- 实现 Tauri 命令和 API
- 集成 RustChat 核心功能

### 3. 前后端通信
- 使用 `@tauri-apps/api` 进行前后端通信
- 通过 Tauri 命令系统调用 Rust 函数
- 事件系统处理实时更新

## 后续开发计划

### Phase 1: 基础 UI 框架 ✅
- [x] Tauri + Svelte 项目初始化
- [x] 开发环境配置
- [x] 热重载和构建流程

### Phase 2: 核心功能集成
- [ ] 集成 RustChat 核心库 (`rustchat-core`)
- [ ] WebSocket 连接管理
- [ ] 用户认证界面
- [ ] 基础聊天界面

### Phase 3: 高级功能
- [ ] 实时消息同步
- [ ] 文件传输
- [ ] 主题系统
- [ ] 设置和配置

### Phase 4: 优化和发布
- [ ] 性能优化
- [ ] 打包和分发
- [ ] 自动更新机制

## 调试和测试

### 前端调试
- 使用浏览器开发者工具 (F12)
- Svelte DevTools 扩展

### 后端调试
- Rust 日志输出到控制台
- 使用 `tauri::command` 进行 API 测试

### 构建生产版本
```bash
npm run tauri build
```

构建产物位于 `src-tauri/target/release/bundle/`

## 注意事项

1. **Workspace 集成**: 项目已集成到主 Cargo workspace
2. **依赖管理**: 前端使用 npm，后端使用 Cargo
3. **热重载**: 监听所有相关 crates 的变更
4. **跨平台**: Tauri 支持 Windows、macOS、Linux

## 相关链接

- [Tauri 文档](https://tauri.app/)
- [Svelte 文档](https://svelte.dev/)
- [Vite 文档](https://vitejs.dev/)
