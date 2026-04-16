# S3 连接问题诊断指南

## 运行诊断测试

### 方法 1: 使用脚本（推荐）
```bash
cd s3_service
bash ../test_s3_connection.sh
```

### 方法 2: 直接运行测试
```bash
cargo test --package s3_service --lib tests::diagnose_s3_connection --features "integration-test" -- --nocapture
```

### 方法 3: 运行所有集成测试
```bash
cargo test --package s3_service --features "integration-test" -- --nocapture
```

## 常见问题排查

### 1. 网络连接问题

**症状**: `dispatch failure`, `connection refused`

**检查项**:
- MinIO 服务是否正在运行？
  ```bash
  # Linux/Mac
  ps aux | grep minio
  
  # Docker
  docker ps | grep minio
  ```
  
- 端点 URL 是否正确？
  ```toml
  # config/app_config.toml
  [s3]
  endpoint = "http://101.33.75.40:19000"  # 检查 IP 和端口
  ```

- 网络是否可达？
  ```bash
  ping 101.33.75.40
  curl http://101.33.75.40:19000/minio/health/live
  ```

### 2. 认证问题

**症状**: `Access Denied`, `InvalidAccessKeyId`

**检查项**:
- Access Key 和 Secret Key 是否正确？
  ```toml
  [s3]
  access_key = "minioadmin"
  secret_key = "REDACTED_S3_SECRET_KEY_V2"
  ```
  
- MinIO 用户权限是否足够？
  ```bash
  # 使用 mc 客户端检查
  mc admin user list local
  mc admin policy list local
  ```

### 3. TLS/SSL 问题

**症状**: `certificate verify failed`, `invalid certificate`

**解决方案**:
- 使用 HTTP 而非 HTTPS（MinIO 默认）
  ```toml
  endpoint = "http://101.33.75.40:19000"  # 注意是 http://
  ```

- 如果使用 HTTPS，确保证书有效

### 4. 区域配置问题

**症状**: `InvalidRegion`, `bucket region mismatch`

**检查项**:
- 区域配置是否正确？
  ```toml
  region = "us-east-1"  # MinIO 通常使用 us-east-1
  ```

### 5. 桶名冲突

**症状**: `BucketAlreadyExists`, `BucketAlreadyOwnedByYou`

**解决方案**:
- 桶名可能已被占用，更改桶名
  ```toml
  default_bucket = "rust-my-app-v2"
  ```

### 6. 路径风格问题

**症状**: DNS 解析失败

**解决方案**:
- MinIO 必须使用路径风格
  ```toml
  force_path_style = true  # 必须为 true
  ```

## MinIO 配置检查

### 检查 MinIO 配置
```bash
# 使用 mc 客户端
mc admin config get local

# 检查服务状态
mc admin info local
```

### 创建必需的桶
```bash
# 手动创建桶
mc mb local/rust-my-app
mc mb local/chat-file-preview
mc mb local/chat-file-origin

# 设置桶策略（可选）
mc anonymous set download local/chat-file-preview
```

## 测试结果解读

### 成功标志
```
✓ S3 客户端创建成功
✓ 连接成功！
✓ 默认桶已存在（或创建成功）
✓ 文件上传成功
✓ 文件下载成功
```

### 失败标志及处理

#### dispatch failure
- **原因**: 网络连接失败
- **处理**: 检查 MinIO 服务、防火墙、网络

#### Access Denied
- **原因**: 认证失败或权限不足
- **处理**: 检查 Access Key/Secret Key，确认用户权限

#### InvalidBucketName
- **原因**: 桶名不符合规范
- **处理**: 使用小写字母、数字、连字符，长度 3-63

## 高级诊断

### 启用详细日志
```bash
RUST_LOG=debug cargo test --package s3_service --features "integration-test" -- --nocapture
```

### 手动测试 S3 API
```bash
# 使用 curl 测试
curl -v http://101.33.75.40:19000

# 使用 AWS CLI
aws s3 ls --endpoint-url http://101.33.75.40:19000 \
  --no-sign-request
```

### 检查端口占用
```bash
# Linux/Mac
lsof -i :19000
netstat -tunlp | grep 19000

# Windows
netstat -ano | findstr :19000
```

## 联系支持

如果以上方法都无法解决问题，请提供：
1. 诊断测试完整输出
2. MinIO 版本和配置
3. 网络环境信息（防火墙、代理等）
4. 错误日志（启用 RUST_LOG=debug）