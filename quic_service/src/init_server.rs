use std::collections::HashMap;
use std::fs;
use toml::Value;
use entity::read_config;
use crate::{quic_client, quic_server};

///初始化服务
pub async fn start_server() -> anyhow::Result<()> {
    // 本地客户端
    let addr = "127.0.0.1:4433".parse()?;
    tokio::spawn(async move {
        quic_client::run_client(addr).await;
    });
    // 读取配置文件内容
    let config_content = fs::read_to_string("./config/app_config.toml")?;
    // 解析配置文件内容
    let config_value: Value = config_content.parse()?;

    // 将解析后的配置转换为 HashMap
    let config_map: HashMap<String, Value> = config_value.try_into()?;
    // 定义服务器监听地址
    let addr = read_config!(config_map, ("quic_server"), "address");
    quic_server::init_server(addr.parse()?);

    Ok(())
}
