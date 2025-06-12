use crate::common::quic_network_service::models::text_msg::TextQuicMsg;

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
    // 2.插入用户消息记录表
    // 3.插入用户消息未读表
    // 4.用户未读消息redis count + 1
    // 5.关闭事务
    Ok(())
}