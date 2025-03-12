use std::iter::Successors;
//创建统一返回对象
use crate::serde_json_to_string;
use serde::{Serialize};

// 定义响应体结构
#[derive(Serialize)]
pub struct CommonResponse<T>
where
    T: Serialize,
{
    pub(crate) code: u32,
    pub(crate) data: T,
    pub(crate) message: String,
}

impl<T> CommonResponse<T>
where
    T: Serialize,
{
    pub fn new(code: u32, data: T, message: String) -> CommonResponse<T> {
        CommonResponse {
            code,
            data,
            message,
        }
    }

    pub fn success(data: T) -> CommonResponse<T> {
        CommonResponse::new(200, data, "Success".to_string())
    }

    pub fn error(data: T, message: String) -> CommonResponse<T> {
        CommonResponse::new(500, data, message)
    }

    pub fn success_json(data: T) -> Result<String, String> {
        serde_json_to_string!(&CommonResponse::success(data))
    }

    pub fn error_json(data: T, message: String) -> Result<String, String> {
        serde_json_to_string!(&CommonResponse::error(data, message))
    }
}

#[derive(Serialize)]
pub struct CommonResponseRef<'a, T>
where
    T: Serialize,
{
    pub(crate) code: u32,
    pub(crate) data: &'a T,
    pub(crate) message: &'a str,
}

impl<'a, T> CommonResponseRef<'a, T>
where
    T: Serialize,
{
    pub fn new(code: u32, data: &'a T, message: &'a str) -> CommonResponseRef<'a, T> {
        CommonResponseRef {
            code,
            data,
            message,
        }
    }

    pub fn success(data: &'a T) -> CommonResponseRef<'a, T> {
        CommonResponseRef::new(200, data, "Success")
    }

    pub fn error(data: &'a T, message: &'a str) -> CommonResponseRef<'a, T> {
        CommonResponseRef::new(500, data, message)
    }

    pub fn success_json(data: &'a T) -> Result<String, String> {
        serde_json_to_string!(&CommonResponseRef::success(data))
    }

    pub fn error_json(data: &'a T, message: &'a str) -> Result<String, String> {
        serde_json_to_string!(&CommonResponseRef::error(data, message))
    }
}
