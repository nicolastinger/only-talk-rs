//创建统一返回对象
use serde::{Deserialize, Serialize};
// 定义响应体结构
#[derive(Serialize, Deserialize)]
struct CommonResponse<T>
where
    T: Serialize
{
    code: u8,
    data: T,
    message: String
}