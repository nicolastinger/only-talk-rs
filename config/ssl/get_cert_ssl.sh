#!/bin/bash

# ======================
# 🔧 请在此处配置你的域名和邮箱（必填！）
# ======================
DOMAIN="onlytalk.cn"      # 替换为你的域名（如：example.com）
EMAIL="2737484812@qq.com"        # 替换为你的注册邮箱
# ======================

# 检查是否以root运行
if [ "$(id -u)" != "0" ]; then
    echo "❌ 错误：请用 sudo 运行此脚本！"
    echo "示例：sudo ./letsencrypt.sh"
    exit 1
fi

# 安装 certbot（如果未安装）
if ! command -v certbot &> /dev/null; then
    echo "🔍 正在安装 certbot..."
    apt update && apt install -y certbot
fi

# 验证配置是否填写
if [ -z "$DOMAIN" ] || [ -z "$EMAIL" ]; then
    echo "❌ 错误：DOMAIN 或 EMAIL 未设置！请编辑脚本开头的配置区。"
    echo "  请将 DOMAIN 和 EMAIL 替换为你的实际值"
    exit 1
fi

# 申请证书（standalone模式 + 非交互式）
echo "🚀 正在申请证书（请确保80端口空闲，脚本会临时占用）..."
certbot certonly --standalone -d "$DOMAIN" --email "$EMAIL" --agree-tos --non-interactive

# 检查申请结果
if [ $? -ne 0 ]; then
    echo "❌ 证书申请失败！常见原因："
    echo "  1. 80端口被占用（检查：sudo lsof -i :80）"
    echo "  2. 域名未解析到本机IP"
    exit 1
fi

# 复制证书到当前目录（关键！）
echo "✅ 证书已生成，正在复制到当前目录..."
cp /etc/letsencrypt/live/"$DOMAIN"/fullchain.pem .
cp /etc/letsencrypt/live/"$DOMAIN"/privkey.pem .

echo -e "\n🎉 证书已成功保存到当前目录！"
echo "  - fullchain.pem（证书链，可直接用于服务）"
echo "  - privkey.pem（私钥，务必保密！）"
echo "💡 提示：私钥文件不要上传到GitHub！"
