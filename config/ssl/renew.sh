#!/bin/bash
DOMAIN="$DOMAIN"  # 从主脚本继承的域名
certbot renew --quiet --post-hook "cp /etc/letsencrypt/live/\$DOMAIN/fullchain.pem . && cp /etc/letsencrypt/live/\$DOMAIN/privkey.pem ."
