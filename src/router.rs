use std::{convert::Infallible, error::Error};

use http::Method;
use touche::{server::Service, Body, Request, Response, StatusCode};

use crate::{
    handler::{Handler, HandlerService},
    response::IntoResponse,
};

trait RoutedService: Service + Send + Sync {
    fn clone_box(&self) -> Box<dyn RoutedService<Body = Self::Body, Error = Self::Error>>;
}

impl<T> RoutedService for T
where
    T: Service + Send + Sync + Clone + 'static,
{
    fn clone_box(&self) -> Box<dyn RoutedService<Body = T::Body, Error = T::Error>> {
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

pub struct MethodRouter<B = Body, E = Infallible> {
    get: Option<Route<B, E>>,
    post: Option<Route<B, E>>,
    put: Option<Route<B, E>>,
    patch: Option<Route<B, E>>,
    delete: Option<Route<B, E>>,
}

macro_rules! impl_method_router_methods {
    ($method:ident) => {
        impl MethodRouter {
            pub fn $method<H, T>(self, handler: H) -> MethodRouter
            where
                H: Handler<T>,
                H: Send + Sync,
                T: Send + Sync + Clone + 'static,
            {
                Self {
                    $method: Some(Route {
                        svc: Box::new(HandlerService::new(handler)),
                    }),
                    ..self
                }
            }
        }
    };
    ($($method:ident),*) => {
        $(impl_method_router_methods!($method);)*
    }
}

macro_rules! impl_router_methods {
    ($method:ident) => {
        pub fn $method<H, T>(handler: H) -> MethodRouter
        where
            H: Handler<T>,
            H: Send + Sync,
            T: Send + Sync + Clone + 'static,
        {
            MethodRouter {
                $method: Some(Route {
                    svc: Box::new(HandlerService::new(handler)),
                }),
                ..Default::default()
            }
        }
    };
    ($($method:ident),*) => {
        $(impl_router_methods!($method);)*
    }
}

impl_method_router_methods!(get, post, put, patch, delete);
impl_router_methods!(get, post, put, patch, delete);

impl Default for MethodRouter {
    fn default() -> Self {
        Self {
            get: None,
            post: None,
            put: None,
            patch: None,
            delete: None,
        }
    }
}

impl Clone for MethodRouter {
    fn clone(&self) -> Self {
        Self {
            get: self.get.clone(),
            post: self.post.clone(),
            put: self.put.clone(),
            patch: self.patch.clone(),
            delete: self.delete.clone(),
        }
    }
}

#[derive(Clone, Default)]
pub struct Router {
    router: matchit::Router<MethodRouter>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            router: matchit::Router::new(),
        }
    }
}

impl Router {
    pub fn route(mut self, path: &str, route: MethodRouter) -> Router {
        self.router.insert(path, route).unwrap();
        self
    }
}

impl Service for Router {
    // TODO: return a BoxedBody so we can accept routes with distinct HttpBody implementations
    type Body = Body;
    type Error = Box<dyn Error + Send + Sync>;

    fn call(&self, request: Request<Body>) -> Result<Response<Self::Body>, Self::Error> {
        let path = request.uri().path();
        match self.router.at(path) {
            Ok(route) => match *request.method() {
                Method::GET if route.value.get.is_some() => {
                    Ok(route.value.get.clone().unwrap().svc.call(request)?)
                }
                Method::POST if route.value.post.is_some() => {
                    Ok(route.value.post.clone().unwrap().svc.call(request)?)
                }
                Method::PUT if route.value.put.is_some() => {
                    Ok(route.value.put.clone().unwrap().svc.call(request)?)
                }
                Method::PATCH if route.value.patch.is_some() => {
                    Ok(route.value.patch.clone().unwrap().svc.call(request)?)
                }
                Method::DELETE if route.value.delete.is_some() => {
                    Ok(route.value.delete.clone().unwrap().svc.call(request)?)
                }
                _ => Ok(StatusCode::METHOD_NOT_ALLOWED.into_response()),
            },
            Err(_) => Ok(StatusCode::NOT_FOUND.into_response()),
        }
    }
}
