# s3_service crate

对象存储抽象层，封装 AWS S3 SDK，支持 MinIO / 阿里云 OSS / AWS S3 三种提供商，提供统一的存储接口、预签名 URL、分片上传、批量操作等能力。

## 职责

- **统一存储抽象**：`StorageBackend` trait 定义上传/下载/删除/列举/复制/预签名等操作，`S3Storage` 为其 S3 实现
- **多提供商**：通过 `S3Provider` 枚举区分 minio / aliyun_oss / aws_s3，配置驱动切换
- **S3 客户端管理**：`S3Client` 封装 aws_sdk_s3，`GlobalS3Client` 全局单例；`S3Config` 从全局配置加载（含 bucket 划分：默认桶、聊天原文件/预览桶、用户/群头像桶）
- **高级操作**：分片上传（multipart）、预签名 URL、批量删除、对象复制/移动、元数据与标签、bucket CORS
- **本地存储已移除**：原 `LocalStorage` 实现随 `/resources` 静态目录一起删除，当前仅保留 S3 后端

## 依赖

- `common`（配置读取）、`aws-sdk-s3`、`aws-config`、`tokio`、`serde`
- 被 `http_service`（文件上传/下载/链接）与 `api`（集成服务）依赖

## 结构

```
s3_service/src/
├── lib.rs                # 模块声明 + 类型再导出
├── client.rs             # S3Client 封装 + GlobalS3Client 全局单例
├── config.rs             # S3Config（provider/endpoint/密钥/bucket 划分/分片参数）+ 全局配置加载
├── error.rs              # S3Error 统一错误类型
├── storage.rs            # StorageBackend trait + S3Storage 实现 + StorageInfo/ObjectInfo
├── tests.rs              # 单元测试（配置解析、错误类型等）
└── operations/           # bucket / upload / download / delete / list / copy_move / metadata /
                          #   presigned / multipart 各操作的具体实现
```

## 关键约定

- `S3Config::from_global_config()` 从 `app_config.toml` 的 `[s3]` 段读取，支持 `${VAR}` 环境变量替换。
- MinIO 必须 `force_path_style = true`。
- 头像桶视为公开桶（返回直接 URL），其余桶返回预签名 URL（`presign_expire_seconds` 控制有效期）。
- 超过 `multipart_threshold` 自动走分片上传。
- `crates/s3_service/tests/minio_integration_test.rs` 是 MinIO 集成测试，需要运行中的 MinIO，非 hermetic；默认不随 `cargo test` 运行。

## 部署形态

仅作为库被上层使用，无独立可执行入口。S3 未启用时上层（`AppState.s3`）为 `None`。
