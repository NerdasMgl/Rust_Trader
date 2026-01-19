#!/bin/bash

# 停止脚本如果出现错误
set -e

echo "🚀 [1/5] 开始初始化 Rust Trader 服务器环境 (Ubuntu)..."

# 1. 更新系统并安装基础依赖 (编译 Rust 需要 build-essential 和 libssl-dev)
echo "📦 [2/5] 更新系统并安装基础依赖..."
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    curl \
    git \
    unzip \
    htop \
    tmux

# 2. 安装 Docker & Docker Compose
if ! command -v docker &> /dev/null; then
    echo "🐳 [3/5] 安装 Docker..."
    sudo apt-get install -y docker.io docker-compose
    sudo systemctl enable --now docker
    # 将当前用户加入 docker 组，避免每次都输 sudo
    sudo usermod -aG docker $USER
    echo "✅ Docker 安装完成"
else
    echo "✅ Docker 已存在，跳过"
fi

# 3. 安装 Rust (使用 Rustup)
if ! command -v cargo &> /dev/null; then
    echo "🦀 [4/5] 安装 Rust 工具链..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo "✅ Rust 安装完成"
else
    echo "✅ Rust 已存在，跳过"
fi

# 4. 配置 Swap (防止轻量云内存不足导致编译崩溃)
# 检查是否已有 swap，如果没有且内存小于 4GB，则创建 4GB swap
TOTAL_MEM=$(grep MemTotal /proc/meminfo | awk '{print $2}')
if [ "$TOTAL_MEM" -lt 4000000 ] && [ ! -f /swapfile ]; then
    echo "💾 [5/5] 检测到内存较小，创建 4GB Swap 空间以防编译崩溃..."
    sudo fallocate -l 4G /swapfile
    sudo chmod 600 /swapfile
    sudo mkswap /swapfile
    sudo swapon /swapfile
    # 写入 fstab 确保重启生效
    echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab
    echo "✅ Swap 创建完成"
fi

echo "🎉 服务器环境初始化完成！"
echo "⚠️  请务必执行 'exit' 退出 SSH 并重新登录，以使 Docker 权限生效。"