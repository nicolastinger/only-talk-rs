//http传入实体校验
#[macro_export]
macro_rules! validate_and_respond {
    ($model:expr) => {{
        if let Err(errors) = $model.validate() {
            return actix_web::HttpResponse::BadRequest().json(errors);
        }
        $model.into_inner()
    }};
}