#!/bin/bash

# ======================
# 🔧 在此配置域名和邮箱（必填！）
# ======================
DOMAIN="onlytalk.cn"      # 替换为你的域名（如：blog.example.com）
EMAIL="2737484812@qq.com" # 替换为你的邮箱（用于证书过期提醒）
# ======================

# 强制使用 bash 运行（防止用 sh 执行时 echo -e 等语法失效）
if [ -z "$BASH_VERSION" ]; then
    exec bash "$0" "$@"
fi

# 检查是否以 root 运行
if [ "$(id -u)" != "0" ]; then
    echo "❌ 错误：请用 sudo 运行此脚本！"
    echo "示例：sudo ./auto_general.sh"
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

# 申请证书（standalone 模式 + 非交互式，已有有效证书则跳过）
echo "🚀 正在申请证书（请确保80端口空闲，脚本会临时占用）..."
certbot certonly --standalone -d "$DOMAIN" --email "$EMAIL" \
    --agree-tos --non-interactive --keep-until-expiring

if [ "$?" -ne 0 ]; then
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
echo ""
echo "⚠️ 正在修复证书权限（安全提示：私钥不应给所有用户可读！）"
chmod 644 fullchain.pem   # 证书链：所有人可读
chmod 640 privkey.pem     # 私钥：仅 root 可读/写，所属组可读

# ======================
# 🌟 自动续期配置
# ======================
echo ""
echo "✨ 正在配置自动续期..."

# 脚本所在目录的绝对路径
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# 生成续期脚本（嵌入具体域名）
cat > "${SCRIPT_DIR}/renew.sh" << EOF
#!/bin/bash
certbot renew --quiet \
    --post-hook "cp /etc/letsencrypt/live/${DOMAIN}/fullchain.pem '${SCRIPT_DIR}/' && cp /etc/letsencrypt/live/${DOMAIN}/privkey.pem '${SCRIPT_DIR}/'"
EOF
chmod +x "${SCRIPT_DIR}/renew.sh"

# 清理旧版本遗留的 /etc/cron.d 任务（旧版用 cron.d，新版改用 root crontab）
rm -f /etc/cron.d/letsencrypt-renew

# 写入 root 的 crontab（每3天0点执行，certbot 内部判断是否真的需要续期）
# 先去掉同一脚本的旧条目，避免重复；再用根用户的 crontab - 读取 stdin 写入
( crontab -l 2>/dev/null | grep -v "${SCRIPT_DIR}/renew.sh"; \
  echo "0 0 */3 * * ${SCRIPT_DIR}/renew.sh" ) | crontab -

echo "✅ 自动续期已配置：每3天自动检查续期，证书复制到：${SCRIPT_DIR}"

# 提示用户
echo ""
echo "🎉 证书申请 & 自动续期已完成！"
echo "  - 证书文件：fullchain.pem, privkey.pem（在当前目录）"
echo "  - 续期脚本：renew.sh（可手动执行：./renew.sh）"
echo "  - 测试续期：./renew.sh --dry-run"
echo ""
echo "💡 提示："
echo "  1. 检查定时任务：sudo crontab -l"
echo "  2. 私钥文件不要上传到 GitHub！"
