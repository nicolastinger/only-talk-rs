//创建统一返回对象 
use serde::{Serialize};
use serde_json;

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

    pub fn success_json(data: T) -> serde_json::Result<String> {
        serde_json::to_string(&CommonResponse::success(data))
    }

    pub fn error_json(data: T, message: String) -> serde_json::Result<String> {
        serde_json::to_string(&CommonResponse::error(data, message))
    }
}

#[derive(Serialize)]
pub struct CommonResponseRef<'a, T>
where
    T: Serialize,
{
    pub(crate) code: u16,
    pub(crate) data: Option<&'a T>,
    pub(crate) message: &'a str,
}

impl<'a, T> CommonResponseRef<'a, T>
where
    T: Serialize,
{
    pub fn new(code: u16, data: Option<&'a T>, message: &'a str) -> Self {
        Self {
            code,
            data,
            message: message.into(),
        }
    }

    pub fn success(data: &'a T) -> CommonResponseRef<'a, T> {
        CommonResponseRef::new(200, Option::from(data), "Success")
    }

    pub fn success_json(data: &'a T) -> serde_json::Result<String> {
        serde_json::to_string(&CommonResponseRef::success(data))
    }
}


#[derive(Serialize)]
pub struct CommonResponseNoDataRef<'a>
{
    pub(crate) code: u16,
    pub data: u8,
    pub(crate) message: &'a str,
}
impl<'a> CommonResponseNoDataRef<'a> {
    pub fn error_json(message: &'a str) -> String {
        serde_json::to_string(&CommonResponseNoDataRef { code: 500, data: 0, message }).unwrap_or_else(|_| "{code:500,message:\"json Panic!\"}".to_string())
    }

    pub fn success_empty() -> String {
        serde_json::to_string(&CommonResponseNoDataRef { code: 204, data: 0, message: "" }).unwrap_or_else(|_| "{code:500,message:\"json Panic!\"}".to_string())
    }
}
