use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use rbatis::crud;
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

impl GroupMessageRecordRead {
    #[rbatis::py_sql("select * from group_message_record_read where group_uuid = #{group_uuid} and read_user = #{read_user} order by timestamp desc limit 1")]
    async fn select_by_group_and_user(rb: &dyn Executor, group_uuid: &Uuid, read_user: &Uuid) -> Vec<GroupMessageRecordRead> {}

    #[rbatis::py_sql("select * from group_message_record_read where group_uuid = #{group_uuid} order by timestamp desc")]
    async fn select_by_group(rb: &dyn Executor, group_uuid: &Uuid) -> Vec<GroupMessageRecordRead> {}
}
