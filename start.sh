#!/bin/bash
# RustChat 快速启动脚本 (Linux/macOS)
# 使用方法: ./start.sh

set -e

echo "🚀 RustChat 快速启动脚本"
echo "========================"

# 检查Rust和Cargo是否安装
if ! command -v cargo &> /dev/null; then
    echo "❌ 错误: 未找到 cargo。请先安装 Rust: https://rustup.rs/"
    exit 1
fi

echo "📦 构建项目..."
cargo build --release

echo "🖥️  启动服务器..."
if command -v gnome-terminal &> /dev/null; then
    # Ubuntu/Debian 使用 gnome-terminal
    gnome-terminal -- bash -c "echo '🖥️ RustChat 服务器'; cargo run --bin rustchatd --release; read -p 'Press Enter to exit...'"
elif command -v xterm &> /dev/null; then
    # 使用 xterm
    xterm -e "echo '🖥️ RustChat 服务器'; cargo run --bin rustchatd --release; read -p 'Press Enter to exit...'" &
elif command -v osascript &> /dev/null; then
    # macOS 使用 Terminal.app
    osascript -e 'tell application "Terminal" to do script "cd \"$(pwd)\" && echo \"🖥️ RustChat 服务器\" && cargo run --bin rustchatd --release"'
else
    echo "⚠️  无法自动打开新终端，请手动启动服务器："
    echo "   cargo run --bin rustchatd --release"
    exit 1
fi

echo "⏱️  等待服务器启动..."
sleep 3

echo "💻 启动客户端..."
for i in 1 2; do
    if command -v gnome-terminal &> /dev/null; then
        gnome-terminal -- bash -c "echo '💻 RustChat 客户端 #$i'; cargo run --bin rustchat-cli --release; read -p 'Press Enter to exit...'" &
    elif command -v xterm &> /dev/null; then
        xterm -e "echo '💻 RustChat 客户端 #$i'; cargo run --bin rustchat-cli --release; read -p 'Press Enter to exit...'" &
    elif command -v osascript &> /dev/null; then
        osascript -e "tell application \"Terminal\" to do script \"cd \\\"$(pwd)\\\" && echo \\\"💻 RustChat 客户端 #$i\\\" && cargo run --bin rustchat-cli --release\""
    fi
done

echo "✅ 启动完成！"
echo "💡 提示:"
echo "   • 在客户端输入 /nick <昵称> 设置昵称"
echo "   • 输入 /help 查看所有命令"
echo "   • 输入 /quit 退出程序"
echo "   • 试试 @echo hello 与机器人聊天"
