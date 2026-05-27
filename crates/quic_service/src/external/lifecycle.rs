use async_trait::async_trait;

use super::state::{ServiceError, ServiceState};

/// 服务生命周期 trait。
/// 定义了服务从创建到销毁的完整生命周期方法。
#[async_trait]
pub trait ServiceLifecycle: Send + Sync {
    /// 服务名称
    fn name(&self) -> &str;

    /// 初始化资源：解析配置、创建端点、启动后台监控等。
    /// 状态必须为 Uninitialized。
    async fn init(&mut self) -> Result<(), ServiceError>;

    /// 启动服务主循环（非阻塞，在后台运行）。
    /// 状态必须为 Initializing 或 Running。
    async fn start(&self) -> Result<(), ServiceError>;

    /// 优雅关闭：发送关闭信号，等待后台任务完成。
    /// 状态必须为 Running。
    async fn stop(&self) -> Result<(), ServiceError>;

    /// 返回当前生命周期状态
    fn status(&self) -> ServiceState;
}
