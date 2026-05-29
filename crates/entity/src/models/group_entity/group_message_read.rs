use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct GroupMessageRecordRead {
    pub id: Option<i64>,
    pub nano_id: Option<String>,
    pub timestamp: Option<i64>,
    pub send_user: Uuid,
    pub group_uuid: Uuid,
    pub read_user: Uuid,
}

crud!(GroupMessageRecordRead {});

impl_select!(GroupMessageRecordRead {select_by_group_and_user(group_uuid: &Uuid, read_user: &Uuid) => r#"where group_uuid = #{group_uuid} and read_user = #{read_user} order by timestamp desc limit 1"#});
impl_select!(GroupMessageRecordRead {select_by_group(group_uuid: &Uuid) => r#"where group_uuid = #{group_uuid} order by timestamp desc"#});
