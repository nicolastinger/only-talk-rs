//http传入实体校验
#[macro_export]
macro_rules! validate_and_respond {
    ($model:expr) => {{
        use validator::Validate;
        let value = $model.into_inner();
        if let Err(errors) = value.validate() {
            return actix_web::HttpResponse::BadRequest().json(errors);
        }
        value
    }};
    ($model:expr, $model_type:expr) => {{
        use validator::Validate;
        let value = $model.into_inner();

        match &value.data{
            Some(data) => {
                if let Err(errors) = data.validate() {
                  return actix_web::HttpResponse::BadRequest().json(errors);
                }
                value
            },
            None => {
                return actix_web::HttpResponse::InternalServerError().finish();
            }
        }

    }};
}

#[macro_export]
macro_rules! respond_to_json {
    ($model:expr) => {{
        use crate::utils::http_response::CommonResponse;

        match $model {
            Ok(t) => actix_web::HttpResponse::Ok()
                .body(serde_json::to_string(&CommonResponse::success(t)).unwrap()),
            Err(t) => HttpResponse::BadRequest().body(
                serde_json::to_string(&CommonResponse::error(t, "error".to_string())).unwrap(),
            ),
        }
    }};
    ($model:expr, $error_str:expr) => {{
        use crate::utils::http_response::CommonResponse;

        match $model {
            Ok(t) => actix_web::HttpResponse::Ok()
                .body(serde_json::to_string(&CommonResponse::success(t)).unwrap()),
            Err(t) => HttpResponse::BadRequest()
                .body(serde_json::to_string(&CommonResponse::error(t, $error_str)).unwrap()),
        }
    }};
}
