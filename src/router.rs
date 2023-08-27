use std::{convert::Infallible, error::Error};

use http::Method;
use matchit::Match;
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
    options: Option<Route<B, E>>,
    trace: Option<Route<B, E>>,
    head: Option<Route<B, E>>,
    connect: Option<Route<B, E>>,
    fallback: Option<Route<B, E>>,
}

impl MethodRouter {
    pub fn merge(&mut self, router: MethodRouter) {
        macro_rules! merge_methods {
            ($method:ident) => {
                if self.$method.is_none() && router.$method.is_some() {
                    self.$method = router.$method;
                } else if self.$method.is_some() && router.$method.is_some() {
                    panic!("Method already defined")
                }
            };
            ($($method:ident),*) => {
                $(merge_methods!($method);)*
            }
        }
        merge_methods!(get, post, put, patch, delete, head, options, trace, connect, fallback);
    }
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

impl_method_router_methods!(get, post, put, patch, delete, head, options, trace, connect);
impl_router_methods!(get, post, put, patch, delete, head, options, trace, connect);

impl MethodRouter {
    pub fn any<H, T>(self, handler: H) -> MethodRouter
    where
        H: Handler<T>,
        H: Send + Sync,
        T: Send + Sync + Clone + 'static,
    {
        MethodRouter {
            fallback: Some(Route {
                svc: Box::new(HandlerService::new(handler)),
            }),
            ..self
        }
    }
}

pub fn any<H, T>(handler: H) -> MethodRouter
where
    H: Handler<T>,
    H: Send + Sync,
    T: Send + Sync + Clone + 'static,
{
    MethodRouter {
        fallback: Some(Route {
            svc: Box::new(HandlerService::new(handler)),
        }),
        ..Default::default()
    }
}

impl Default for MethodRouter {
    fn default() -> Self {
        Self {
            get: None,
            post: None,
            put: None,
            patch: None,
            delete: None,
            options: None,
            trace: None,
            head: None,
            connect: None,
            fallback: None,
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
            options: self.options.clone(),
            trace: self.trace.clone(),
            head: self.head.clone(),
            connect: self.connect.clone(),
            fallback: self.fallback.clone(),
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
        match self.router.at_mut(path) {
            Ok(Match {
                value: existing_route,
                ..
            }) => {
                existing_route.merge(route);
            }
            _ => {
                self.router.insert(path, route).unwrap();
            }
        }
        self
    }
}

impl Service for Router {
    // TODO: return a BoxedBody so we can accept routes with distinct HttpBody implementations
    type Body = Body;
    type Error = Box<dyn Error + Send + Sync>;

    fn call(&self, mut req: Request<Body>) -> Result<Response<Self::Body>, Self::Error> {
        match self.router.at(req.uri().path()) {
            Ok(Match {
                value: route,
                params,
            }) => {
                let params = params
                    .iter()
                    .map(|(k, v)| (k.to_owned(), v.to_owned()))
                    .collect::<Vec<_>>();
                req.extensions_mut().insert(params);
                match *req.method() {
                    Method::GET if route.get.is_some() => {
                        Ok(route.get.clone().unwrap().svc.call(req)?)
                    }
                    Method::POST if route.post.is_some() => {
                        Ok(route.post.clone().unwrap().svc.call(req)?)
                    }
                    Method::PUT if route.put.is_some() => {
                        Ok(route.put.clone().unwrap().svc.call(req)?)
                    }
                    Method::PATCH if route.patch.is_some() => {
                        Ok(route.patch.clone().unwrap().svc.call(req)?)
                    }
                    Method::DELETE if route.delete.is_some() => {
                        Ok(route.delete.clone().unwrap().svc.call(req)?)
                    }
                    Method::HEAD if route.head.is_some() => {
                        Ok(route.head.clone().unwrap().svc.call(req)?)
                    }
                    Method::OPTIONS if route.options.is_some() => {
                        Ok(route.options.clone().unwrap().svc.call(req)?)
                    }
                    Method::TRACE if route.trace.is_some() => {
                        Ok(route.trace.clone().unwrap().svc.call(req)?)
                    }
                    Method::CONNECT if route.connect.is_some() => {
                        Ok(route.connect.clone().unwrap().svc.call(req)?)
                    }
                    _ if route.fallback.is_some() => {
                        Ok(route.fallback.clone().unwrap().svc.call(req)?)
                    }
                    _ => Ok(StatusCode::METHOD_NOT_ALLOWED.into_response()),
                }
            }
            Err(_) => Ok(StatusCode::NOT_FOUND.into_response()),
        }
    }
}
