use std::sync::Arc;

use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_s3::Client as AwsS3Client;
use aws_sdk_s3::config::BehaviorVersion;
use tracing::info;

use crate::config::S3Config;
use crate::error::S3Error;

/// S3 客户端包装结构体
///
/// 该结构体封装了 AWS SDK 的 S3 客户端，提供统一的 S3 服务访问接口。
/// 内部持有底层 AWS SDK 客户端实例和配置信息。
///
/// # 字段
///
/// - `inner`: 底层 AWS SDK S3 客户端实例
/// - `config`: S3 服务配置
///
/// # 示例
///
/// ```rust,no_run
/// use s3_service::{S3Client, S3Config};
///
/// async fn example() -> Result<(), Box<dyn std::error::Error>> {
///     let config = S3Config::default_minio();
///     let client = S3Client::new(config).await?;
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct S3Client {
    /// AWS S3 SDK 客户端实例
    /// 用于实际的 S3 API 调用
    pub inner: AwsS3Client,

    /// S3 配置信息
    /// 包含端点、认证信息、默认桶等
    pub config: S3Config,
}

impl S3Client {
    /// 创建新的 S3 客户端实例
    ///
    /// 根据提供的配置初始化 AWS SDK S3 客户端。
    /// 支持 MinIO、阿里云 OSS 和 AWS S3 存储服务。
    ///
    /// # 参数
    ///
    /// - `config`: S3 配置，包括认证信息、端点、区域等
    ///
    /// # 返回值
    ///
    /// 返回初始化完成的 S3 客户端实例，失败时返回错误
    ///
    /// # 初始化流程
    ///
    /// 1. 根据存储类型设置凭证提供者名称
    /// 2. 创建 AWS 凭证对象
    /// 3. 构建 S3 SDK 配置（区域、凭证、路径风格等）
    /// 4. 如果配置了自定义端点，则设置端点 URL
    /// 5. 创建 SDK 客户端并记录初始化日志
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use s3_service::{S3Client, S3Config};
    ///
    /// async fn create_client() -> Result<S3Client, s3_service::S3Error> {
    ///     let config = S3Config::default_minio();
    ///     S3Client::new(config).await
    /// }
    /// ```
    pub async fn new(config: S3Config) -> Result<Self, S3Error> {
        // 根据存储服务类型设置凭证提供者名称标识
        let provider_name: &'static str = match config.provider {
            crate::config::S3Provider::MinIO => "minio",
            crate::config::S3Provider::AliyunOSS => "aliyun_oss",
            crate::config::S3Provider::AwsS3 => "aws_s3",
        };

        // 创建 AWS 凭证对象
        let credentials = Credentials::new(
            &config.access_key_id,
            &config.secret_access_key,
            None, // 无会话令牌
            None, // 无过期时间
            provider_name,
        );

        // 构建 S3 SDK 配置
        let mut s3_config_builder = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest()) // 使用最新行为版本
            .region(Region::new(config.region.clone())) // 设置区域
            .credentials_provider(credentials) // 设置凭证
            .force_path_style(config.force_path_style); // 路径风格访问（MinIO 必需）

        // 如果配置了自定义端点，则设置端点 URL
        // 用于 MinIO、阿里云 OSS 等非 AWS S3 服务
        if !config.endpoint_url.is_empty() {
            s3_config_builder = s3_config_builder.endpoint_url(&config.endpoint_url);
        }

        // 构建配置并创建客户端
        let s3_config = s3_config_builder.build();
        let inner = AwsS3Client::from_conf(s3_config);

        // 记录初始化成功日志
        info!(
            "S3 客户端初始化完成 - Provider: {}, Endpoint: {}, Bucket: {}",
            config.provider, config.endpoint_url, config.default_bucket
        );

        Ok(S3Client { inner, config })
    }

    /// 获取默认桶名称
    ///
    /// 从配置中返回默认桶名称。
    /// 当未指定桶时，使用默认桶执行操作。
    ///
    /// # 返回值
    ///
    /// 默认桶名称的字符串引用
    pub fn default_bucket(&self) -> &str {
        &self.config.default_bucket
    }

    /// S3 服务健康检查
    ///
    /// 通过尝试列出所有桶来检查 S3 服务的可用性。
    /// 这是一种轻量级的健康检查方法。
    ///
    /// # 返回值
    ///
    /// - `Ok(true)`: 服务可用
    /// - `Ok(false)`: 服务不可用
    /// - `Err`: 检查过程中出错
    ///
    /// # 说明
    ///
    /// 即使健康检查失败，服务仍可能部分可用（例如权限问题）
    pub async fn health_check(&self) -> Result<bool, S3Error> {
        match self.inner.list_buckets().send().await {
            Ok(_) => {
                info!("S3 健康检查通过");
                Ok(true)
            }
            Err(e) => {
                tracing::error!("S3 健康检查失败: {:?}", e);
                Ok(false)
            }
        }
    }

    /// 确保默认桶存在
    ///
    /// 检查默认桶是否存在，不存在则创建。
    /// 该操作是幂等的，可安全地多次调用。
    ///
    /// # 返回值
    ///
    /// 成功返回 `Ok(())`，失败返回错误
    ///
    /// # 工作流程
    ///
    /// 1. 使用 `head_bucket` 检查桶是否存在
    /// 2. 如果桶不存在，调用 `create_bucket` 创建
    /// 3. 记录操作日志
    ///
    /// # 错误情况
    ///
    /// - 权限不足
    /// - 桶名已被其他账号占用
    /// - 网络连接失败
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use s3_service::S3Client;
    ///
    /// async fn setup_bucket(client: &S3Client) -> Result<(), s3_service::S3Error> {
    ///     client.ensure_default_bucket().await
    /// }
    /// ```
    pub async fn ensure_default_bucket(&self) -> Result<(), S3Error> {
        self.ensure_bucket(&self.config.default_bucket).await
    }

    /// 确保所有配置的桶存在
    ///
    /// 检查并创建（如果缺失）配置中引用的每个桶：
    /// `default_bucket`、`chat_file_preview_bucket`、`chat_file_origin_bucket`、
    /// `user_avatar_bucket` 和 `group_avatar_bucket`。
    ///
    /// 头像桶（`user_avatar_bucket` / `group_avatar_bucket`）被当作公开桶，
    /// 并设置公开读策略，以便其 `public_url()` 链接可以访问。
    ///
    /// 该操作是幂等的，可安全地在每次启动时调用。
    ///
    /// # 返回值
    ///
    /// 成功返回 `Ok(())`，失败返回错误
    pub async fn ensure_all_buckets(&self) -> Result<(), S3Error> {
        let mut buckets = vec![
            &self.config.default_bucket,
            &self.config.chat_file_preview_bucket,
            &self.config.chat_file_origin_bucket,
            &self.config.user_avatar_bucket,
            &self.config.group_avatar_bucket,
        ];
        // 去重，避免对相同桶名重复执行 HEAD/CREATE
        buckets.dedup();
        for bucket in buckets {
            self.ensure_bucket(bucket).await?;
        }

        // 头像桶需公开读，否则 public_url 无法直接访问
        for bucket in [&self.config.user_avatar_bucket, &self.config.group_avatar_bucket] {
            self.ensure_public_read_policy(bucket).await?;
        }
        Ok(())
    }

    /// 确保单个桶存在，不存在则创建
    async fn ensure_bucket(&self, bucket: &str) -> Result<(), S3Error> {
        // 检查桶是否存在
        // head_bucket 成功表示桶存在且我们有访问权限
        let exists = self.inner.head_bucket().bucket(bucket).send().await.is_ok();

        if !exists {
            info!("S3 桶 {} 不存在,正在创建...", bucket);
            // 创建桶
            self.inner
                .create_bucket()
                .bucket(bucket)
                .send()
                .await
                .map_err(|e| S3Error::AwsError(format!("Failed to create bucket: {}", e)))?;
            info!("S3 桶 {} 创建成功", bucket);
        } else {
            info!("S3 桶 {} 已存在", bucket);
        }

        Ok(())
    }

    /// 确保桶具有公开读策略（幂等）
    ///
    /// 向匿名主体授予 `s3:GetObject` 权限，使对象可以通过 `public_url()` 公开访问。
    async fn ensure_public_read_policy(&self, bucket: &str) -> Result<(), S3Error> {
        let policy_str = format!(
            "{{\"Version\":\"2012-10-17\",\"Statement\":[{{\"Effect\":\"Allow\",\"Principal\":\"*\",\"Action\":[\"s3:GetObject\"],\"Resource\":[\"arn:aws:s3:::{}/*\"]}}]}}",
            bucket
        );
        self.inner.put_bucket_policy().bucket(bucket).policy(&policy_str).send().await.map_err(
            |e| {
                S3Error::AwsError(format!(
                    "Failed to set public policy on bucket {}: {}",
                    bucket, e
                ))
            },
        )?;
        info!("S3 桶 {} 已设置公开读策略", bucket);
        Ok(())
    }
}

/// 全局 S3 客户端单例管理器
///
/// 提供全局单例 S3 客户端的初始化和管理功能。
/// 使用 Arc 实现线程安全的共享访问。
///
/// # 设计模式
///
/// 使用单例模式确保整个应用使用同一个 S3 客户端实例，
/// 避免重复创建连接，提高资源利用率。
pub struct GlobalS3Client;

impl GlobalS3Client {
    /// 初始化全局 S3 客户端
    ///
    /// 创建 S3 客户端并初始化默认桶（如果启用）。
    /// 返回 Arc 包装的客户端实例，可在多线程间共享。
    ///
    /// # 参数
    ///
    /// - `config`: S3 配置
    ///
    /// # 返回值
    ///
    /// 返回 Arc 包装的 S3 客户端实例
    ///
    /// # 初始化流程
    ///
    /// 1. 根据配置创建 S3 客户端
    /// 2. 如果配置中启用了 S3，则确保所有配置的桶存在
    /// 3. 返回 Arc 包装的客户端实例
    ///
    /// # 错误处理
    ///
    /// - 客户端创建失败返回错误
    /// - 桶创建失败仅记录警告日志，不影响服务启动
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use s3_service::{GlobalS3Client, S3Client, S3Config};
    /// use std::sync::Arc;
    ///
    /// async fn init_s3() -> Result<Arc<S3Client>, s3_service::S3Error> {
    ///     let config = S3Config::default_minio();
    ///     GlobalS3Client::init(config).await
    /// }
    /// ```
    pub async fn init(config: S3Config) -> Result<Arc<S3Client>, S3Error> {
        // 创建 S3 客户端实例
        let client = S3Client::new(config).await?;

        // 如果启用了 S3 服务，则确保所有配置的桶存在
        if client.config.enabled
            && let Err(e) = client.ensure_all_buckets().await
        {
            // 桶创建失败仅记录警告日志，服务仍将继续启动
            tracing::warn!("确保 S3 桶存在失败: {},服务仍将继续启动", e);
        }

        // 返回 Arc 包装的客户端，支持多线程共享
        Ok(Arc::new(client))
    }
}
