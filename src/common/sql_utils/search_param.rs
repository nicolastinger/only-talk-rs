use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ShortParams {
    pub param_str_1: Option<String>,
    pub param_str_2: Option<String>,
    pub param_str_3: Option<String>,
    pub param_str_4: Option<String>,
    pub param_int_1: Option<i32>,
    pub param_int_2: Option<i32>,
    pub param_int_3: Option<i64>,
    pub param_int_4: Option<i64>
}
