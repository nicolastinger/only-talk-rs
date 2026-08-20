use std::sync::Arc;

use common::state::CoreState;
use email_service::manager::EmailManager;
use s3_service::S3Client;

/// HTTP 服务应用状态,通过 `web::Data<AppState>` 注入一次。
#[derive(Clone)]
pub struct AppState {
    pub core: CoreState,
    pub s3: Arc<S3Client>,
    pub email: Arc<EmailManager>,
}

impl AppState {
    pub fn db(&self) -> &rbatis::RBatis {
        &self.core.db
    }

    pub fn redis(&self) -> &deadpool_redis::Pool {
        &self.core.redis
    }

    pub fn s3(&self) -> &Arc<S3Client> {
        &self.s3
    }
}
