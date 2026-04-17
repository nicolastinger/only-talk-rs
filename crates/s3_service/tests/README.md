# S3 Service 测试说明

## 测试结构

本测试套件包含两个部分：

### 1. 单元测试 (`src/tests.rs`)
测试不需要外部服务的基础功能：
- 配置解析
- 错误处理
- 枚举类型
- 默认配置

**运行单元测试：**
```bash
cargo test -p s3_service --lib
```

### 2. 集成测试 (`tests/minio_integration_test.rs`)
测试与 MinIO 服务的完整集成功能：
- 客户端初始化和健康检查
- 文件上传、下载、删除
- 范围下载（断点续传）
- 文件列举、复制、移动
- 批量操作
- 预签名 URL
- 大文件分片上传
- 元数据管理

**前置条件：**
- MinIO 服务已启动并可访问
- MinIO 配置在 `config/app_config.toml` 中正确设置
- 默认配置：
  - 端点：http://101.33.75.40:19000
  - 访问密钥：minioadmin
  - 密钥：REDACTED_S3_SECRET_KEY_V2
  - 默认桶：rust-my-app

**运行集成测试：**
```bash
# 运行所有集成测试
cargo test -p s3_service --test minio_integration_test

# 运行特定测试
cargo test -p s3_service --test minio_integration_test test_upload_and_download

# 运行所有测试（包括单元测试和集成测试）
cargo test -p s3_service
```

## 测试覆盖范围

### ✅ 已覆盖功能

#### 客户端管理
- [x] 客户端初始化
- [x] 健康检查
- [x] 确保默认桶存在

#### 基础操作
- [x] 文件上传
- [x] 文件下载
- [x] 文件删除
- [x] 文件列举
- [x] 文件复制
- [x] 文件移动

#### 高级功能
- [x] 范围下载（支持断点续传）
- [x] 批量删除
- [x] 预签名 URL（GET）
- [x] 大文件分片上传
- [x] 元数据管理

#### 错误处理
- [x] 删除不存在的对象
- [x] 下载不存在的对象

#### 配置验证
- [x] S3Provider 解析
- [x] 默认配置验证
- [x] 错误类型转换

## 测试最佳实践

1. **测试隔离**：每个测试使用唯一的文件路径前缀（如 `test/integration/`）
2. **清理资源**：测试完成后自动清理上传的文件
3. **幂等性**：测试可以重复运行，不会因遗留数据失败
4. **异步测试**：所有集成测试使用 `#[tokio::test]`

## 持续集成

可以将这些测试集成到 CI/CD 流程中：

```yaml
# GitHub Actions 示例
- name: Run S3 Service Tests
  run: cargo test -p s3_service
  env:
    # 如果需要覆盖配置，可以设置环境变量
    MINIO_ENDPOINT: http://localhost:9000
```

## 性能测试

集成测试包含大文件上传测试（15MB），可用于验证分片上传功能。

## 故障排查

如果测试失败，请检查：
1. MinIO 服务是否运行：`curl http://101.33.75.40:19000/minio/health/live`
2. 网络连接是否正常
3. 认证信息是否正确
4. 存储桶权限是否足够

## 未来改进

- [ ] 添加并发上传测试
- [ ] 添加预签名 URL PUT 测试
- [ ] 添加跨桶复制测试
- [ ] 添加性能基准测试
- [ ] 添加 Mock 测试（不依赖真实服务）