use actix_web::{body::MessageBody, dev::{ServiceRequest, ServiceResponse}, middleware::{Next}, Error, HttpMessage};
use crate::utils::jwt_util::{decode_jwt};

pub async fn error_record_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    // pre-processing

    // 获取请求的方法和路径
    let method = req.method().clone();
    let path = req.path().to_string();

    if path.contains("user/test_token/get") {
       return next.call(req).await;
    }

    let authorization = req.headers().clone();
    let authorization = authorization.get("Authorization");

    let token = match authorization {
        None => {
            return Err(actix_web::error::ErrorUnauthorized("Unauthorized"))
        }
        Some(token) => token.to_str().unwrap().to_string(),
    };
    let account = decode_jwt(token)?;
    // 如果需要读取请求体，可以使用 `take_payload` 方法
    // 注意：这会消耗请求体，所以只有在必要时才这样做
    // let payload = req.take_payload();

    // 调用下一个中间件或处理程序
    let res = next.call(req).await?;

    // post-processing

    // 访问响应的状态码
    let status = res.status();

    // 你可以在这里对响应进行修改或者记录日志
    println!("{} {} completed with status: {}", method, path, status);

    // 如果你需要访问响应体，可以使用 `response.into_body()` 方法
    // 注意：这会消耗响应体，所以只有在必要时才这样做
    // let body = res.into_body();
    // let bytes = body.try_into_bytes().unwrap_or_else(|_| Bytes::new());

    // 返回原始响应
    Ok(res)
}
 
