use quinn::SendStream;

pub struct CommonSender {
    pub quic_sender: Option<SendStream>,
    pub websocket_sender: Option<String>,
    pub recv_target: String
}

impl CommonSender {
    pub fn quic_new(quic_send_connection: SendStream) -> Self {
        CommonSender{
           quic_sender: Some(quic_send_connection),
           websocket_sender: None,
           recv_target: String::new()
        }
    }

    pub fn change_recv_target(&mut self, recv_target: String) {
        self.recv_target = recv_target;
    }
}