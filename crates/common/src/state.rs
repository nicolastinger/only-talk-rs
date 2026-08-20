use deadpool_redis::Pool;
use rbatis::RBatis;

/// 共享连接状态,当前供 HTTP 服务使用,后续 QUIC 服务也可复用。
#[derive(Clone)]
pub struct CoreState {
    pub db: RBatis,
    pub redis: Pool,
}
