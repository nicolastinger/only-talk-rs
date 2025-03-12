use rbatis::crud;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Friend {
    pub uuid: Option<String>
}

crud!(Friend {});


