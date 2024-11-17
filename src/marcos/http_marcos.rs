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
}