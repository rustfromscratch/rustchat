# RustChat 快速启动脚本 (Windows PowerShell)
# 使用方法: .\start.ps1

Write-Host "🚀 RustChat 快速启动脚本" -ForegroundColor Cyan
Write-Host "========================" -ForegroundColor Cyan

# 检查Rust和Cargo是否安装
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "❌ 错误: 未找到 cargo。请先安装 Rust: https://rustup.rs/" -ForegroundColor Red
    exit 1
}

Write-Host "📦 构建项目..." -ForegroundColor Yellow
try {
    cargo build --release
    Write-Host "✅ 构建成功!" -ForegroundColor Green
} catch {
    Write-Host "❌ 构建失败: $_" -ForegroundColor Red
    exit 1
}

Write-Host "🖥️  启动服务器..." -ForegroundColor Yellow
Start-Process powershell -ArgumentList @(
    "-NoExit",
    "-Command",
    "Write-Host '🖥️ RustChat 服务器' -ForegroundColor Green; cargo run --bin rustchatd --release"
)

Write-Host "⏱️  等待服务器启动..." -ForegroundColor Yellow
Start-Sleep -Seconds 3

Write-Host "💻 启动客户端..." -ForegroundColor Yellow
for ($i = 1; $i -le 2; $i++) {
    Start-Process powershell -ArgumentList @(
        "-NoExit", 
        "-Command", 
        "Write-Host '💻 RustChat 客户端 #$i' -ForegroundColor Blue; cargo run --bin rustchat-cli --release"
    )
    Start-Sleep -Seconds 1
}

Write-Host "✅ 启动完成！" -ForegroundColor Green
Write-Host "💡 提示:" -ForegroundColor Cyan
Write-Host "   • 在客户端输入 /nick <昵称> 设置昵称" -ForegroundColor White
Write-Host "   • 输入 /help 查看所有命令" -ForegroundColor White
Write-Host "   • 输入 /quit 退出程序" -ForegroundColor White
Write-Host "   • 试试 @echo hello 与机器人聊天" -ForegroundColor White

Write-Host "`n按任意键退出..." -ForegroundColor Gray
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
