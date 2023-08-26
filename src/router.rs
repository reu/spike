use std::{convert::Infallible, error::Error};

use touche::{server::Service, Body, Request, Response, StatusCode};

use crate::{
    handler::{Handler, HandlerService},
    response::IntoResponse,
};

trait RoutedService: Service + Send + Sync {
    fn clone_box(&self) -> Box<dyn RoutedService<Body = Self::Body, Error = Self::Error> + Send>;
}

impl<T> RoutedService for T
where
    T: Service + Send + Sync + Clone + 'static,
{
    fn clone_box(&self) -> Box<dyn RoutedService<Body = T::Body, Error = T::Error> + Send> {
        Box::new(self.clone())
    }
}

pub struct Route<B = Body, E = Infallible> {
    svc: Box<dyn RoutedService<Body = B, Error = E>>,
}

impl Clone for Route {
    fn clone(&self) -> Self {
        Route {
            svc: self.svc.clone_box(),
        }
    }
}

#[derive(Clone, Default)]
pub struct Router {
    router: matchit::Router<Route>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            router: matchit::Router::new(),
        }
    }
}

pub fn on<H, T>(handler: H) -> Route
where
    H: Handler<T>,
    H: Sync + Send,
    T: Send + Sync + Clone + 'static,
{
    Route {
        svc: Box::new(HandlerService::new(handler)),
    }
}

impl Router {
    pub fn route(mut self, path: &str, route: Route) -> Router {
        self.router.insert(path, route).unwrap();
        self
    }
}

impl Service for Router {
    type Body = Body;
    type Error = Box<dyn Error + Send + Sync>;

    fn call(&self, request: Request<Body>) -> Result<Response<Self::Body>, Self::Error> {
        let path = request.uri().path();
        match self.router.at(path) {
            Ok(route) => Ok(route.value.clone().svc.call(request)?),
            Err(_) => Ok(StatusCode::NOT_FOUND.into_response()),
        }
    }
}
