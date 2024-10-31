use crate::module::chat_msg_mod::model::common_send_connection::CommonSender;

trait CommonMsg {
    async fn send_text_msg(&self, msg: &str, sender: CommonSender);
}