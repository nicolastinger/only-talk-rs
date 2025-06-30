
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