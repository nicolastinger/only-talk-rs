use actix_web::{HttpResponse, Responder, delete, get, post, web};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

pub fn s3_service_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/s3")
            .service(get_download_presigned_url_api)
            .service(get_upload_presigned_url_api)
            .service(list_buckets_api)
            .service(create_bucket_api)
            .service(delete_bucket_api)
            .service(bucket_exists_api)
            .service(list_objects_api)
            .service(delete_object_api)
            .service(delete_objects_batch_api)
            .service(copy_object_api)
            .service(move_object_api)
            .service(get_object_metadata_api)
            .service(get_object_tags_api)
            .service(put_object_tags_api)
            .service(put_bucket_cors_api)
            .service(s3_health_check_api),
    );
}

// ==================== DTO ====================

#[derive(Debug, Deserialize)]
pub struct PresignedUrlRequest {
    pub key: String,
    pub bucket: Option<String>,
    pub expires_seconds: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct PresignedUrlResponse {
    pub url: String,
    pub key: String,
    pub expires_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct CreateBucketRequest {
    pub bucket: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteObjectsRequest {
    pub keys: Vec<String>,
    pub bucket: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CopyMoveRequest {
    pub source_key: String,
    pub dest_key: String,
    pub source_bucket: Option<String>,
    pub dest_bucket: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListObjectsRequest {
    pub prefix: Option<String>,
    pub max_keys: Option<i32>,
    pub bucket: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PutCorsRequest {
    pub bucket: String,
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub max_age_seconds: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct PutTagsRequest {
    pub key: String,
    pub bucket: Option<String>,
    pub tags: std::collections::HashMap<String, String>,
}

// ==================== Helpers ====================

fn get_bucket(client: &s3_service::S3Client, bucket_opt: Option<&String>) -> String {
    bucket_opt.cloned().unwrap_or_else(|| client.config.default_bucket.clone())
}

// 第三方宏 serde_json::json! 内部会对 to_value 的结果调用 unwrap,并非本项目代码
#[allow(clippy::disallowed_methods)]
fn ok_response<T: Serialize>(data: &T) -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "code": 200,
        "data": data,
        "message": "Success"
    }))
}

// 第三方宏 serde_json::json! 内部会对 to_value 的结果调用 unwrap,并非本项目代码
#[allow(clippy::disallowed_methods)]
fn ok_msg(msg: &str) -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "code": 200,
        "message": msg
    }))
}

// 第三方宏 serde_json::json! 内部会对 to_value 的结果调用 unwrap,并非本项目代码
#[allow(clippy::disallowed_methods)]
fn err_response(msg: &str) -> HttpResponse {
    HttpResponse::BadRequest().json(serde_json::json!({
        "code": 500,
        "message": msg
    }))
}

/// S3 客户端为启动时强制要求,始终可用
fn require_s3(state: &web::Data<AppState>) -> &s3_service::S3Client {
    state.s3()
}

// ==================== API ====================

#[get("/health")]
async fn s3_health_check_api(state: web::Data<AppState>) -> impl Responder {
    let client = require_s3(&state);
    let healthy = client.health_check().await.unwrap_or(false);
    if healthy { ok_msg("S3服务正常") } else { err_response("S3服务不可用") }
}

#[post("/presign/download")]
async fn get_download_presigned_url_api(
    state: web::Data<AppState>,
    body: web::Json<PresignedUrlRequest>,
) -> impl Responder {
    let client = require_s3(&state);
    let bucket = get_bucket(client, body.bucket.as_ref());
    let expires_secs = body.expires_seconds.unwrap_or(client.config.presign_expire_seconds);
    let expires = std::time::Duration::from_secs(expires_secs);

    match s3_service::operations::presigned::generate_download_presigned_url(
        client, &bucket, &body.key, expires,
    )
    .await
    {
        Ok(url) => ok_response(&PresignedUrlResponse {
            url,
            key: body.key.clone(),
            expires_seconds: expires_secs,
        }),
        Err(e) => err_response(&e.to_string()),
    }
}

#[post("/presign/upload")]
async fn get_upload_presigned_url_api(
    state: web::Data<AppState>,
    body: web::Json<PresignedUrlRequest>,
) -> impl Responder {
    let client = require_s3(&state);
    let bucket = get_bucket(client, body.bucket.as_ref());
    let expires_secs = body.expires_seconds.unwrap_or(client.config.presign_expire_seconds);
    let expires = std::time::Duration::from_secs(expires_secs);

    match s3_service::operations::presigned::generate_upload_presigned_url(
        client, &bucket, &body.key, expires,
    )
    .await
    {
        Ok(url) => ok_response(&PresignedUrlResponse {
            url,
            key: body.key.clone(),
            expires_seconds: expires_secs,
        }),
        Err(e) => err_response(&e.to_string()),
    }
}

#[get("/buckets")]
async fn list_buckets_api(state: web::Data<AppState>) -> impl Responder {
    let client = require_s3(&state);
    match s3_service::operations::bucket::list_buckets(client).await {
        Ok(buckets) => ok_response(&buckets),
        Err(e) => err_response(&e.to_string()),
    }
}

#[post("/buckets")]
async fn create_bucket_api(
    state: web::Data<AppState>,
    body: web::Json<CreateBucketRequest>,
) -> impl Responder {
    let client = require_s3(&state);
    match s3_service::operations::bucket::create_bucket(client, &body.bucket).await {
        Ok(_) => ok_msg("存储桶创建成功"),
        Err(e) => err_response(&e.to_string()),
    }
}

#[delete("/buckets/{bucket}")]
async fn delete_bucket_api(state: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let client = require_s3(&state);
    let bucket = path.into_inner();
    match s3_service::operations::bucket::delete_bucket(client, &bucket).await {
        Ok(_) => ok_msg("存储桶删除成功"),
        Err(e) => err_response(&e.to_string()),
    }
}

#[get("/buckets/{bucket}/exists")]
// 第三方宏 serde_json::json! 内部会对 to_value 的结果调用 unwrap,并非本项目代码
#[allow(clippy::disallowed_methods)]
async fn bucket_exists_api(state: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let client = require_s3(&state);
    let bucket = path.into_inner();
    match s3_service::operations::bucket::bucket_exists(client, &bucket).await {
        Ok(exists) => ok_response(&serde_json::json!({"exists": exists})),
        Err(e) => err_response(&e.to_string()),
    }
}

#[post("/objects/list")]
async fn list_objects_api(
    state: web::Data<AppState>,
    body: web::Json<ListObjectsRequest>,
) -> impl Responder {
    let client = require_s3(&state);
    let bucket = get_bucket(client, body.bucket.as_ref());
    match s3_service::operations::list::list_objects(
        client,
        &bucket,
        body.prefix.as_deref(),
        body.max_keys,
    )
    .await
    {
        Ok(objects) => ok_response(&objects),
        Err(e) => err_response(&e.to_string()),
    }
}

#[delete("/objects/{key}")]
async fn delete_object_api(
    state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<Option<String>>,
) -> impl Responder {
    let client = require_s3(&state);
    let key = path.into_inner();
    let bucket = query.into_inner().unwrap_or_else(|| client.config.default_bucket.clone());
    match s3_service::operations::delete::delete_object(client, &bucket, &key).await {
        Ok(_) => ok_msg("对象删除成功"),
        Err(e) => err_response(&e.to_string()),
    }
}

#[post("/objects/delete_batch")]
async fn delete_objects_batch_api(
    state: web::Data<AppState>,
    body: web::Json<DeleteObjectsRequest>,
) -> impl Responder {
    let client = require_s3(&state);
    let bucket = get_bucket(client, body.bucket.as_ref());
    let keys: Vec<&str> = body.keys.iter().map(|s| s.as_str()).collect();
    match s3_service::operations::delete::delete_objects(client, &bucket, &keys).await {
        Ok(result) => ok_response(&result),
        Err(e) => err_response(&e.to_string()),
    }
}

#[post("/objects/copy")]
async fn copy_object_api(
    state: web::Data<AppState>,
    body: web::Json<CopyMoveRequest>,
) -> impl Responder {
    let client = require_s3(&state);
    let source_bucket = get_bucket(client, body.source_bucket.as_ref());
    let dest_bucket = get_bucket(client, body.dest_bucket.as_ref());
    match s3_service::operations::copy_move::copy_object(
        client,
        &source_bucket,
        &body.source_key,
        &dest_bucket,
        &body.dest_key,
    )
    .await
    {
        Ok(_) => ok_msg("对象复制成功"),
        Err(e) => err_response(&e.to_string()),
    }
}

#[post("/objects/move")]
async fn move_object_api(
    state: web::Data<AppState>,
    body: web::Json<CopyMoveRequest>,
) -> impl Responder {
    let client = require_s3(&state);
    let source_bucket = get_bucket(client, body.source_bucket.as_ref());
    let dest_bucket = get_bucket(client, body.dest_bucket.as_ref());
    match s3_service::operations::copy_move::move_object(
        client,
        &source_bucket,
        &body.source_key,
        &dest_bucket,
        &body.dest_key,
    )
    .await
    {
        Ok(_) => ok_msg("对象移动成功"),
        Err(e) => err_response(&e.to_string()),
    }
}

#[get("/objects/{key}/metadata")]
async fn get_object_metadata_api(
    state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<Option<String>>,
) -> impl Responder {
    let client = require_s3(&state);
    let key = path.into_inner();
    let bucket = query.into_inner().unwrap_or_else(|| client.config.default_bucket.clone());
    match s3_service::operations::metadata::head_object(client, &bucket, &key).await {
        Ok(metadata) => ok_response(&metadata),
        Err(e) => err_response(&e.to_string()),
    }
}

#[get("/objects/{key}/tags")]
async fn get_object_tags_api(
    state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<Option<String>>,
) -> impl Responder {
    let client = require_s3(&state);
    let key = path.into_inner();
    let bucket = query.into_inner().unwrap_or_else(|| client.config.default_bucket.clone());
    match s3_service::operations::metadata::get_object_tagging(client, &bucket, &key).await {
        Ok(tags) => ok_response(&tags),
        Err(e) => err_response(&e.to_string()),
    }
}

#[post("/objects/tags")]
async fn put_object_tags_api(
    state: web::Data<AppState>,
    body: web::Json<PutTagsRequest>,
) -> impl Responder {
    let client = require_s3(&state);
    let bucket = get_bucket(client, body.bucket.as_ref());
    match s3_service::operations::metadata::put_object_tagging(
        client,
        &bucket,
        &body.key,
        body.tags.clone(),
    )
    .await
    {
        Ok(_) => ok_msg("标签设置成功"),
        Err(e) => err_response(&e.to_string()),
    }
}

#[post("/buckets/cors")]
async fn put_bucket_cors_api(
    state: web::Data<AppState>,
    body: web::Json<PutCorsRequest>,
) -> impl Responder {
    let client = require_s3(&state);
    match s3_service::operations::bucket::put_bucket_cors(
        client,
        &body.bucket,
        body.allowed_origins.clone(),
        body.allowed_methods.clone(),
        body.allowed_headers.clone(),
        body.max_age_seconds,
    )
    .await
    {
        Ok(_) => ok_msg("CORS设置成功"),
        Err(e) => err_response(&e.to_string()),
    }
}
