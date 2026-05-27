#!/bin/bash

# ======================
# 🔧 请在此处配置你的域名和邮箱（必填！）
# ======================
DOMAIN="onlytalk.cn"      # 替换为你的域名（如：blog.example.com）
EMAIL="2737484812@qq.com"        # 替换为你的邮箱（用于证书过期提醒）
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

# 验证配置
if [ -z "$DOMAIN" ] || [ -z "$EMAIL" ]; then
    echo "❌ 错误：DOMAIN 或 EMAIL 未设置！请编辑脚本开头的配置区。"
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

# 复制证书到当前目录
echo "✅ 证书已生成，正在复制到当前目录..."
cp /etc/letsencrypt/live/"$DOMAIN"/fullchain.pem .
cp /etc/letsencrypt/live/"$DOMAIN"/privkey.pem .

# ======================
# 🔐 修复权限（安全重点！）
# ======================
echo -e "\n⚠️ 正在修复证书权限（安全提示：私钥不应给所有用户可读！）"
# 证书链（fullchain.pem）：所有用户可读（644）
chmod 644 fullchain.pem
# 私钥（privkey.pem）
chmod 640 privkey.pem


# ======================
# 🌟 自动续期配置（关键优化！）
# ======================
echo -e "\n✨ 正在配置自动续期..."
# 获取脚本所在目录的绝对路径
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# 生成续期脚本（嵌入具体域名）
cat > "${SCRIPT_DIR}/renew.sh" << EOF
#!/bin/bash
# certbot renew 自动检查所有证书，仅到期才续
certbot renew --quiet --post-hook "cp /etc/letsencrypt/live/${DOMAIN}/fullchain.pem '${SCRIPT_DIR}/' && cp /etc/letsencrypt/live/${DOMAIN}/privkey.pem '${SCRIPT_DIR}/'"
EOF
chmod +x "${SCRIPT_DIR}/renew.sh"

# 生成cron任务（每3天0点执行一次，certbot内部判断是否真需要续期）
echo "0 0 */3 * * root ${SCRIPT_DIR}/renew.sh" > /etc/cron.d/letsencrypt-renew
echo "✅ 自动续期已配置：每3天自动检查续期，证书复制到：${SCRIPT_DIR}"

# 提示用户
echo -e "\n🎉 证书申请 & 自动续期已完成！"
echo "  - 证书文件：fullchain.pem, privkey.pem（在当前目录）"
echo "  - 续期脚本：renew.sh（可手动执行：./renew.sh）"
echo "  - 测试续期：./renew.sh --dry-run"
echo -e "💡 提示：\n  1. 请用 'crontab -l' 检查 cron 任务\n  2. 私钥文件不要上传到 GitHub！"
