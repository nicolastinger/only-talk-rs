use std::sync::Arc;

use crate::chat_node::ChatNode;
use crate::config::ChatNodeConfig;
use crate::lifecycle::ServiceLifecycle;

/// 初始化并启动 QUIC ChatNode 服务
pub async fn start_server() -> anyhow::Result<Arc<ChatNode>> {
    let config = ChatNodeConfig::from_toml("./config/app_config.toml")?;
    let mut node = ChatNode::new(config);
    node.init().await?;
    let node = Arc::new(node);
    node.start().await?;
    Ok(node)
}
