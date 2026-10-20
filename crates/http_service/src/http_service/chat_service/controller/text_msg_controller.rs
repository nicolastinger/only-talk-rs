use actix_web::{HttpRequest, Responder, post, web};

use crate::common::dto::base_dto::AuthAccount;
use crate::common::dto::base_page_dto::BasePageDTO;
use crate::http_service::chat_service::service::text_msg_service::get_chat_by_limit;
use crate::state::AppState;
use crate::utils::http_response::CommonResponseNoDataRef;
use crate::{get_uuid_from_header, respond_json_any};

pub fn text_msg_service(cfg: &mut web::ServiceConfig) {
    cfg.service(get_chat_record_api);
}

/// 获取特定好友的聊天记录(任务08: Body 可选 `session_uuid` 直达会话)
#[post("/get_chat_record/{uuid}")]
pub async fn get_chat_record_api(
    req: HttpRequest,
    state: web::Data<AppState>,
    friend_uuid: web::Path<String>,
    base_page: web::Json<BasePageDTO>,
) -> impl Responder {
    let uuid = get_uuid_from_header!(req);
    let uuid_clone = uuid.clone();
    respond_json_any!(
        get_chat_by_limit(state.db(), uuid_clone, friend_uuid.into_inner(), base_page.into_inner())
            .await
    )
}
