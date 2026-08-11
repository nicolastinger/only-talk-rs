use deadpool_redis::Pool;
use rbatis::RBatis;

/// Shared connection state, used by HTTP service now and reusable by QUIC service later.
#[derive(Clone)]
pub struct CoreState {
    pub db: RBatis,
    pub redis: Pool,
}
