use std::sync::Arc;

use common::state::CoreState;
use email_service::manager::EmailManager;
use s3_service::S3Client;

/// HTTP service application state, injected once via `web::Data<AppState>`.
#[derive(Clone)]
pub struct AppState {
    pub core: CoreState,
    pub s3: Option<Arc<S3Client>>,
    pub email: Arc<EmailManager>,
}

impl AppState {
    pub fn db(&self) -> &rbatis::RBatis {
        &self.core.db
    }

    pub fn redis(&self) -> &deadpool_redis::Pool {
        &self.core.redis
    }

    pub fn s3(&self) -> Option<&Arc<S3Client>> {
        self.s3.as_ref()
    }
}
