# RustChat - 现代化聊天软件

基于Rust的现代化聊天软件，采用渐进式开发策略。

## 🎯 当前版本: v0.1.0 (M0 - "Hello World" 原型)

### ✅ 已实现功能

#### USER-001: UUID用户标识生成
- ✅ UUID用户ID生成和管理
- ✅ 用户配置本地存储 (`~/.rustchat/config.json`)
- ✅ 服务器端为新连接返回UUID
- ✅ 客户端接收并保存用户ID

### 🏗️ 项目结构

```
rustchat/
├── Cargo.toml              # 工作空间配置
├── crates/
│   ├── rustchat-types/     # 共享类型定义
│   │   ├── src/
│   │   │   ├── user.rs     # 用户ID类型
│   │   │   ├── message.rs  # 消息类型
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── rustchat-core/      # 核心功能库
│   │   ├── src/
│   │   │   ├── user.rs     # 用户配置管理
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── rustchat-server/    # WebSocket服务器
│   │   ├── src/
│   │   │   └── main.rs     # 服务器主程序
│   │   └── Cargo.toml
│   └── rustchat-cli/       # 命令行客户端
│       ├── src/
│       │   └── main.rs     # CLI主程序
│       └── Cargo.toml
└── README.md
```

## 🚀 快速开始

### 环境要求

- Rust 1.70+ 
- Cargo

### 构建项目

```bash
# 克隆项目
git clone <repository-url>
cd rustchat

# 构建所有组件
cargo build --release
```

### 运行服务器

```bash
# 启动WebSocket服务器
cargo run --bin rustchatd

# 或者使用发布版本
cargo run --bin rustchatd --release
```

服务器将在以下地址启动：
- WebSocket: `ws://127.0.0.1:8080/ws`
- 健康检查: `http://127.0.0.1:8080/health`

### 运行客户端

在新的终端窗口中：

```bash
# 启动CLI客户端
cargo run --bin rustchat-cli

# 或者使用发布版本
cargo run --bin rustchat-cli --release
```

### 使用说明

1. **首次运行**: 客户端会自动生成UUID并保存到 `~/.rustchat/config.json`
2. **设置昵称**: 输入 `/nick <你的昵称>`
3. **发送消息**: 直接输入文本消息
4. **查看帮助**: 输入 `/help`
5. **退出程序**: 输入 `/quit`

## 🧪 测试双终端聊天

1. 启动服务器：
   ```bash
   cargo run --bin rustchatd
   ```

2. 打开第一个客户端：
   ```bash
   cargo run --bin rustchat-cli
   ```

3. 打开第二个客户端（新终端）：
   ```bash
   cargo run --bin rustchat-cli
   ```

4. 在任一客户端设置昵称并发送消息，另一个客户端会实时收到！

## 🔧 开发特性

### 设计原则
- **低耦合**: 模块化设计，各组件职责单一
- **可扩展**: 预留扩展接口，支持未来功能添加
- **类型安全**: 强类型系统，编译时错误检查
- **异步优先**: 基于Tokio的高性能异步架构

### 核心组件

#### rustchat-types
- 定义共享的数据类型 (`UserId`, `Message`)
- 提供序列化/反序列化支持
- 类型安全的UUID封装

#### rustchat-core  
- 用户配置管理
- 文件系统操作
- 业务逻辑封装

#### rustchat-server
- WebSocket服务器
- 实时消息广播
- 连接管理

#### rustchat-cli
- 命令行界面
- 用户交互处理
- 本地状态管理

## 📝 配置文件

用户配置文件位置: `~/.rustchat/config.json`

示例配置：
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "nickname": "Alice",
  "version": "0.1.0"
}
```

## 🔍 日志和调试

项目使用 `tracing` 进行日志记录。设置环境变量可以调整日志级别：

```bash
# 显示详细日志
RUST_LOG=debug cargo run --bin rustchatd

# 显示特定模块日志
RUST_LOG=rustchat_server=info cargo run --bin rustchatd
```

## 🧪 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定包的测试
cargo test -p rustchat-types
cargo test -p rustchat-core
```