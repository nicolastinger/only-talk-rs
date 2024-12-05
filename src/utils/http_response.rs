use std::iter::Successors;
//创建统一返回对象
use serde::{Deserialize, Serialize};
// 定义响应体结构
#[derive(Serialize, Deserialize)]
pub struct CommonResponse<T>
where
    T: Serialize
{
    pub(crate) code: u32,
    pub(crate) data: T,
    pub(crate) message: String
}

impl<T> CommonResponse<T>
where
    T: Serialize
{
    pub fn new(code: u32, data: T, message: String) -> CommonResponse<T> {
         CommonResponse{
            code,
            data,
            message
        }
    }

    pub fn success(data: T) -> CommonResponse<T> {
        CommonResponse::new(200, data, "Success".to_string())
    }

    pub fn error(data: T, message: String) -> CommonResponse<T> {
        CommonResponse::new(500, data, message)
    }
}