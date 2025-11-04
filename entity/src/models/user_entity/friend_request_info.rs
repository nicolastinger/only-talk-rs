use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select};
use serde::{Deserialize, Serialize};

/// 好友请求表
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FriendRequestInfo {
    pub id: Option<i64>,
    pub uuid: Option<Uuid>,
    /// 0-已发起未处理,1-已接受,2-不接受,3-已拒绝再发起
    pub accept_status: Option<u8>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
    // 请求信息
    pub request_message: Option<String>,
    pub accept_message: Option<String>,
    pub request_user: Option<Uuid>,
    pub accept_user: Option<Uuid>,
    // 添加方式
    pub add_type: Option<String>,
    pub version: Option<u32>,
}

crud!(FriendRequestInfo {});

impl_select!(FriendRequestInfo {select_by_uuid(uuid:&Uuid, last_uuid:&Uuid) => "`where (accept_user = #{uuid} and request_user = #{last_uuid}) or (accept_user = #{last_uuid} and request_user = #{uuid})`"});
