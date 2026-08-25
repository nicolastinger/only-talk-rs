#!/bin/sh
# certbot 容器入口：首次签发证书 + 注册每 3 天续期任务
# 依赖环境变量 DOMAIN / EMAIL（由 docker-compose 注入）
set -e

: "${DOMAIN:?DOMAIN 环境变量未设置}"
: "${EMAIL:?EMAIL 环境变量未设置}"

CERT_DIR=/app/config/ssl
LE_LIVE=/etc/letsencrypt/live

# 确保 cron 可用（certbot/certbot 镜像基于 alpine）
if ! command -v crond >/dev/null 2>&1; then
    apk add --no-cache cron >/dev/null 2>&1 || true
fi

# ---------- 首次申请证书（standalone 模式，需占用 80 端口） ----------
if [ ! -f "${CERT_DIR}/fullchain.pem" ]; then
    echo "[certbot] 首次申请证书: ${DOMAIN}"
    certbot certonly --standalone -d "${DOMAIN}" -m "${EMAIL}" \
        --agree-tos --non-interactive --keep-until-expiring
    cp "${LE_LIVE}/${DOMAIN}/fullchain.pem" "${CERT_DIR}/fullchain.pem"
    cp "${LE_LIVE}/${DOMAIN}/privkey.pem" "${CERT_DIR}/privkey.pem"
    chmod 644 "${CERT_DIR}/fullchain.pem"
    chmod 640 "${CERT_DIR}/privkey.pem"
    echo "[certbot] 证书已复制到 ${CERT_DIR}"
else
    echo "[certbot] 证书已存在，跳过首次申请"
fi

# ---------- 生成续期脚本（嵌域名，避免 cron 环境变量丢失） ----------
cat > /app/renew.sh <<EOF
#!/bin/sh
certbot renew --quiet \
    --deploy-hook "cp ${LE_LIVE}/${DOMAIN}/fullchain.pem ${CERT_DIR}/fullchain.pem && chmod 644 ${CERT_DIR}/fullchain.pem; cp ${LE_LIVE}/${DOMAIN}/privkey.pem ${CERT_DIR}/privkey.pem && chmod 640 ${CERT_DIR}/privkey.pem"
EOF
chmod +x /app/renew.sh

# ---------- 注册定时任务：每 3 天 0 点执行续期 ----------
mkdir -p /etc/crontabs
( grep -v '/app/renew.sh' /etc/crontabs/root 2>/dev/null || true; \
  echo "0 0 */3 * * /app/renew.sh" ) > /etc/crontabs/root
echo "[certbot] 写入续期 crontab（每 3 天）"

exec crond -f -c /etc/crontabs
