use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    HttpMessage,
    Error,
};
use futures_util::future::{ok, LocalBoxFuture, Ready};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct CreatedBy(pub String);

pub struct CreatedByMiddleware;

impl<S, B> Transform<S, ServiceRequest> for CreatedByMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
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
    
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let srv = self.service.clone();
        
        Box::pin(async move {
            if let Some(created_by) = req.match_info().get("created_by") {
                // Store it for handlers
                req.extensions_mut()
                    .insert(CreatedBy(created_by.clone().to_string()));
            }
           
            srv.call(req).await
        })
    }
}