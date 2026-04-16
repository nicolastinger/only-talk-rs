use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures_util::future::{ok, Ready};
use tracing::Instrument;
use uuid::Uuid;

pub struct TraceIdMiddleware;

impl<S, B> Transform<S, ServiceRequest> for TraceIdMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Error = Error;
    type Response = ServiceResponse<B>;
    type Transform = TraceIdMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(TraceIdMiddlewareService { service })
    }
}

pub struct TraceIdMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for TraceIdMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = futures_util::future::LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let trace_id = Uuid::new_v4().to_string();
        
        let span = tracing::info_span!(
            "http_request",
            trace_id = %trace_id,
            method = %req.method(),
            path = %req.path(),
        );

        req.extensions_mut().insert(trace_id.clone());
        
        let fut = self.service.call(req);
        
        Box::pin(async move {
            let res = fut.await?;
            Ok(res)
        }.instrument(span))
    }
}

pub fn get_trace_id(req: &actix_web::HttpRequest) -> Option<String> {
    req.extensions()
        .get::<String>()
        .filter(|s: &&String| s.len() == 36)
        .cloned()
}
