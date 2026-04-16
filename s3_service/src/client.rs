use std::sync::Arc;

use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::BehaviorVersion;
use aws_sdk_s3::Client as AwsS3Client;
use tracing::info;

use crate::config::S3Config;
use crate::error::S3Error;

/// S3客户端封装结构体
///
/// 该结构体封装了AWS SDK的S3客户端,提供了统一的S3服务访问接口。
/// 持有底层的AWS SDK客户端实例和配置信息。
///
/// # 字段说明
///
/// - `inner`: AWS SDK的原始S3客户端实例
/// - `config`: S3服务配置信息
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
    /// AWS S3 SDK客户端实例
    /// 用于执行实际的S3 API调用
    pub inner: AwsS3Client,
    
    /// S3配置信息
    /// 包含端点、认证信息、默认存储桶等配置
    pub config: S3Config,
}

impl S3Client {
    /// 创建新的S3客户端实例
    ///
    /// 根据提供的配置信息初始化AWS SDK的S3客户端。
    /// 支持MinIO、阿里云OSS和AWS S3等多种存储服务。
    ///
    /// # 参数
    ///
    /// - `config`: S3配置信息,包含认证、端点、区域等
    ///
    /// # 返回值
    ///
    /// 返回初始化成功的S3客户端实例,或初始化失败的错误
    ///
    /// # 初始化流程
    ///
    /// 1. 根据provider类型设置凭证提供者名称
    /// 2. 创建AWS凭证对象
    /// 3. 构建S3 SDK配置(区域、凭证、路径风格等)
    /// 4. 如果配置了自定义端点,则设置端点URL
    /// 5. 创建SDK客户端并记录日志
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
        // 根据存储提供者类型设置提供者名称标识
        let provider_name: &'static str = match config.provider {
            crate::config::S3Provider::MinIO => "minio",
            crate::config::S3Provider::AliyunOSS => "aliyun_oss",
            crate::config::S3Provider::AwsS3 => "aws_s3",
        };
        
        // 创建AWS凭证对象
        let credentials = Credentials::new(
            &config.access_key_id,
            &config.secret_access_key,
            None,  // 无会话令牌
            None,  // 无过期时间
            provider_name,
        );

        // 构建S3 SDK配置
        let mut s3_config_builder = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())  // 使用最新行为版本
            .region(Region::new(config.region.clone()))   // 设置区域
            .credentials_provider(credentials)              // 设置凭证
            .force_path_style(config.force_path_style);    // 路径风格访问(MinIO必需)

        // 如果配置了自定义端点,则设置端点URL
        // 用于MinIO、阿里云OSS等非AWS S3服务
        if !config.endpoint_url.is_empty() {
            s3_config_builder = s3_config_builder
                .endpoint_url(&config.endpoint_url);
        }

        // 构建配置并创建客户端
        let s3_config = s3_config_builder.build();
        let inner = AwsS3Client::from_conf(s3_config);

        // 记录初始化成功的日志
        info!(
            "S3客户端初始化成功 - Provider: {}, Endpoint: {}, Bucket: {}",
            config.provider, config.endpoint_url, config.default_bucket
        );

        Ok(S3Client { inner, config })
    }

    /// 获取默认存储桶名称
    ///
    /// 返回配置中设置的默认存储桶名称。
    /// 默认存储桶用于未指定桶名时的操作。
    ///
    /// # 返回值
    ///
    /// 默认存储桶名称的字符串引用
    pub fn default_bucket(&self) -> &str {
        &self.config.default_bucket
    }

    /// S3服务健康检查
    ///
    /// 通过尝试列举所有存储桶来检查S3服务的可用性。
    /// 这是一个轻量级的健康检查方法。
    ///
    /// # 返回值
    ///
    /// - `Ok(true)`: 服务可用
    /// - `Ok(false)`: 服务不可用
    /// - `Err`: 检查过程中发生错误
    ///
    /// # 注意事项
    ///
    /// 即使健康检查失败,服务可能仍然部分可用(如权限不足时)
    pub async fn health_check(&self) -> Result<bool, S3Error> {
        match self.inner.list_buckets().send().await {
            Ok(_) => {
                info!("S3健康检查通过");
                Ok(true)
            }
            Err(e) => {
                tracing::error!("S3健康检查失败: {:?}", e);
                Ok(false)
            }
        }
    }

    /// 确保默认存储桶存在
    ///
    /// 检查默认存储桶是否存在,如果不存在则创建。
    /// 这是一个幂等操作,可以安全地多次调用。
    ///
    /// # 返回值
    ///
    /// 成功返回`Ok(())`,失败返回错误信息
    ///
    /// # 工作流程
    ///
    /// 1. 使用`head_bucket`检查桶是否存在
    /// 2. 如果桶不存在,则调用`create_bucket`创建
    /// 3. 记录操作日志
    ///
    /// # 错误情况
    ///
    /// - 权限不足
    /// - 桶名已被其他账户占用
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
        let bucket = &self.config.default_bucket;
        
        // 检查桶是否存在
        // head_bucket成功表示桶存在且有访问权限
        let exists = self
            .inner
            .head_bucket()
            .bucket(bucket)
            .send()
            .await
            .is_ok();

        if !exists {
            info!("默认存储桶 {} 不存在，正在创建...", bucket);
            // 创建存储桶
            self.inner
                .create_bucket()
                .bucket(bucket)
                .send()
                .await
                .map_err(|e| S3Error::AwsError(format!("创建存储桶失败: {}", e)))?;
            info!("默认存储桶 {} 创建成功", bucket);
        } else {
            info!("默认存储桶 {} 已存在", bucket);
        }

        Ok(())
    }
}

/// 全局S3客户端单例管理器
///
/// 提供全局单例的S3客户端初始化和管理功能。
/// 使用Arc实现线程安全的共享访问。
///
/// # 设计模式
///
/// 采用单例模式,确保整个应用使用同一个S3客户端实例,
/// 避免重复创建连接,提高资源利用效率。
pub struct GlobalS3Client;

impl GlobalS3Client {
    /// 初始化全局S3客户端
    ///
    /// 创建S3客户端并初始化默认存储桶(如果启用)。
    /// 返回Arc包装的客户端实例,可在多个线程间共享。
    ///
    /// # 参数
    ///
    /// - `config`: S3配置信息
    ///
    /// # 返回值
    ///
    /// 返回Arc包装的S3客户端实例
    ///
    /// # 初始化流程
    ///
    /// 1. 根据配置创建S3客户端
    /// 2. 如果配置中启用了S3,则确保默认存储桶存在
    /// 3. 返回Arc包装的客户端实例
    ///
    /// # 错误处理
    ///
    /// - 客户端创建失败会返回错误
    /// - 默认桶创建失败仅记录警告,不影响服务启动
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use s3_service::{GlobalS3Client, S3Config};
    /// use std::sync::Arc;
    ///
    /// async fn init_s3() -> Result<Arc<s3_service::S3Client>, s3_service::S3Error> {
    ///     let config = S3Config::default_minio();
    ///     GlobalS3Client::init(config).await
    /// }
    /// ```
    pub async fn init(config: S3Config) -> Result<Arc<S3Client>, S3Error> {
        // 创建S3客户端实例
        let client = S3Client::new(config).await?;
        
        // 如果S3服务已启用,确保默认桶存在
        if client.config.enabled {
            if let Err(e) = client.ensure_default_bucket().await {
                // 桶创建失败不影响服务启动,仅记录警告
                tracing::warn!("确保默认存储桶失败: {}，服务仍可启动", e);
            }
        }
        
        // 返回Arc包装的客户端,支持多线程共享
        Ok(Arc::new(client))
    }
}
