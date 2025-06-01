//http传入实体校验
#[macro_export]
macro_rules! validate_and_respond {
    ($model:expr) => {{
        use crate::utils::http_response::CommonResponse;
        use validator::Validate;
        let value = $model.into_inner();
        if let Err(errors) = value.validate() {
            return HttpResponse::BadRequest().body(
                serde_json::to_string(&CommonResponse::error(errors, "errorValidate".to_string())).unwrap(),
            );
        }
        value
    }};
    ($model:expr, $model_type:expr) => {{
        use validator::Validate;
        let value = $model.into_inner();

        match &value.data {
            Some(data) => {
                if let Err(errors) = data.validate() {
                    return actix_web::HttpResponse::BadRequest().json(errors);
                }
                value
            }
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

#[macro_export]
macro_rules! respond_json {
    ($model:expr) => {{
        match $model {
            Ok(t) => actix_web::HttpResponse::Ok().body(t),
            Err(t) => HttpResponse::BadRequest().body(t),
        }
    }};
}

#[macro_export]
macro_rules! respond_json_any {
    ($model:expr) => {{
        match $model {
            Ok(t) => actix_web::HttpResponse::Ok().body(t),
            Err(t) => {
                use crate::utils::http_response::CommonResponseNoDataRef;
                use log::error;
                error!("err_context {:?}", t);
                error!("{}", t.backtrace());
                HttpResponse::BadRequest().body(CommonResponseNoDataRef::error_json(&t.to_string()))
            }
        }
    }};
}

#[macro_export]
macro_rules! serde_json_to_string {
    ($model:expr) => {{
        use log::error;
        use rust_i18n::t;

        match serde_json::to_string($model) {
            Ok(t) => Ok(t),
            Err(t) => {
                error!("{}", t.to_string());
                Err(t!("json_serialize_error").to_string())
            }
        }
    }};
}

#[macro_export]
macro_rules! get_account_from_header {
    ($model:expr) => {{
        use actix_web::HttpMessage;
        let map = $model.extensions_mut();
        match map.get::<AuthAccount>() {
            None => None,
            Some(t) => Some(t.to_owned().0),
        }
    }};
}
