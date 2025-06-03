use rbatis::crud;
use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};

/// 用户p2p信息实体
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct UserP2pInfo {
    pub id: Option<u64>,
    pub server_path: Option<String>,  //服务器地址
    pub client_path: Option<String>,  //客户端地址
    pub created_at: Option<i64>,  //创建时间
    pub updated_at: Option<i64>,  //更新时间
    pub status: Option<u8>, //连接状态,0-未连接,1-已连接,2-已失效
    pub finish_at: Option<u64>,  //结束时间
    pub server_user: Option<Uuid>,  //服务端用户id
    pub client_user: Option<Uuid>,  //客户端用户id
    pub enable: Option<bool>,  //是否同意建立连接
    pub json_str: Option<String>,  // json字符串
}

crud!(UserP2pInfo {});