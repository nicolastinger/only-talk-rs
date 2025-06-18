use anyhow::anyhow;
use rbatis::RBatis;
use crate::common::quic_network_service::models::text_msg::TextQuicMsg;
use crate::RBATIS_DATABASE;

/// 用户下线
pub async fn user_offline(uuid: String) -> Result<(), anyhow::Error> {
    // TODO
    // 1.设置redis分布式锁，防止用户下线的同时立马上线
    // 2.同步所有redis缓存到数据库，记录用户操作
    // 3.清理redis缓存，清理redis锁
    Ok(())
}

/// 用户上线
pub async fn user_online(uuid: String) -> Result<(), anyhow::Error> {
    // TODO
    // 1.设置redis分布式锁，防止用户上线的同时立马下线
    // 2.同步所有数据库到redis缓存
    // 3.清理redis锁
    Ok(())
}

/// 用户新增聊天记录
pub async fn add_user_chat_record(text_msg: TextQuicMsg) -> Result<(), anyhow::Error> {
    // TODO
    // 1.开启事务
    let rb = RBATIS_DATABASE.read().await;
    let rb = rb.as_ref().ok_or(anyhow!("获取连接失败"))?;

    // 2. 开启事务
    let mut tx = rb.acquire_begin().await?; // 开启事务并获取事务对象
    
    // 2.插入用户消息记录表
    // 3.插入用户消息未读表
    // 4.用户未读消息redis count + 1
    // 5.关闭事务
    Ok(())
}

pub async fn execute_transaction_operations(rb: &RBatis, text_msg: TextQuicMsg)->Result<(), anyhow::Error>{
    Ok(())
}