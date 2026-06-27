use actix_web::{
    Error, HttpResponse,
    body::{EitherBody, MessageBody},
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
};
use futures_util::future::{LocalBoxFuture, Ready, ok};
use std::rc::Rc;

use crate::models::responses::Response;

pub struct JsonErrorMiddleware;

impl<S, B> Transform<S, ServiceRequest> for JsonErrorMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = JsonErrorMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(JsonErrorMiddlewareService {
            service: Rc::new(service),
        })
    }
}

pub struct JsonErrorMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for JsonErrorMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let srv = self.service.clone();

        Box::pin(async move {
            let res = srv.call(req).await?;

            let status = res.status();

            // Split response
            let (req, res) = res.into_parts();

            // If NOT an error → pass through
            if !status.is_client_error() && !status.is_server_error() {
                return Ok(ServiceResponse::new(req, res.map_into_left_body()));
            }

            // Check Content-Type
            let has_json_body = res
                .headers()
                .get(actix_web::http::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|v| v.contains("application/json"))
                .unwrap_or(false);

            // If handler already returned JSON → DO NOT OVERRIDE
            if has_json_body {
                return Ok(ServiceResponse::new(req, res.map_into_left_body()));
            }

            // Otherwise, fallback JSON error
            let json = HttpResponse::build(status).json(Response {
                success: false,
                message: status.canonical_reason().unwrap_or("Error").to_string(),
                code: status.as_u16(),
                data: None,
                description: String::new(),
                status: String::from("Error"),
            });

            Ok(ServiceResponse::new(req, json.map_into_right_body()))
        })
    }
}
