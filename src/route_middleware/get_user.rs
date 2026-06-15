use actix_web::{
    Error, HttpMessage, HttpResponse,
    body::EitherBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
};
use futures_util::future::{LocalBoxFuture, Ready, ok};
use std::rc::Rc;

use crate::helper::jwt::{decode_username, extract_bearer_token};
use crate::models::responses::Response;

#[derive(Clone, Debug)]
pub struct CreatedBy(pub String);

pub struct CreatedByMiddleware;

impl<S, B> Transform<S, ServiceRequest> for CreatedByMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = CreatedByMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(CreatedByMiddlewareService {
            service: Rc::new(service),
        })
    }
}

pub struct CreatedByMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for CreatedByMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
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
        let username = req
            .headers()
            .get("Authorization")
            .and_then(|value| value.to_str().ok())
            .and_then(extract_bearer_token)
            .and_then(|token| decode_username(token).ok());

        Box::pin(async move {
            match username {
                Some(username) => {
                    req.extensions_mut().insert(CreatedBy(username));
                    let response = srv.call(req).await?;
                    Ok(response.map_into_left_body())
                }
                None => {
                    let (http_req, _) = req.into_parts();
                    let response = HttpResponse::Unauthorized().json(Response {
                        status: "Error".to_string(),
                        code: 401,
                        message: "Unauthorized".to_string(),
                        description: "Missing, malformed, invalid, or expired bearer token"
                            .to_string(),
                        data: None,
                        success: false,
                    });
                    Ok(ServiceResponse::new(
                        http_req,
                        response.map_into_right_body(),
                    ))
                }
            }
        })
    }
}
