use std::{convert::Infallible, io, marker::PhantomData, str::Utf8Error};

use http::{
    request::Parts as RequestParts, response::Parts as ResponseParts, HeaderMap, HeaderValue,
};
use touche::{header, server::Service, Body, HttpBody, Method, Request, Response, StatusCode};

mod macros;

pub trait FromRequest: Sized {
    type Rejection: IntoResponse;

    fn from_request(req: Request<Body>) -> Result<Self, Self::Rejection>;
}

pub trait FromRequestPart: Sized {
    type Rejection: IntoResponse;

    fn from_request_parts(parts: &mut RequestParts) -> Result<Self, Self::Rejection>;
}

impl FromRequestPart for Method {
    type Rejection = Infallible;

    fn from_request_parts(parts: &mut RequestParts) -> Result<Self, Self::Rejection> {
        Ok(parts.method.clone())
    }
}

impl FromRequestPart for HeaderMap {
    type Rejection = Infallible;

    fn from_request_parts(parts: &mut RequestParts) -> Result<Self, Self::Rejection> {
        Ok(parts.headers.clone())
    }
}

pub enum StringRejection {
    Io(io::Error),
    InvalidUtf8(Utf8Error),
}

impl IntoResponse for StringRejection {
    fn into_response(self) -> Response<Body> {
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("error reading body"))
            .unwrap()
    }
}

impl<T> FromRequest for T
where
    T: FromRequestPart,
{
    type Rejection = <Self as FromRequestPart>::Rejection;

    fn from_request(req: Request<Body>) -> Result<Self, Self::Rejection> {
        let (mut parts, _body) = req.into_parts();
        Self::from_request_parts(&mut parts)
    }
}

impl FromRequest for String {
    type Rejection = StringRejection;

    fn from_request(req: Request<Body>) -> Result<Self, Self::Rejection> {
        let body = req.into_body();
        let body = body.into_bytes().map_err(StringRejection::Io)?;
        Ok(std::str::from_utf8(&body)
            .map_err(StringRejection::InvalidUtf8)?
            .to_owned())
    }
}

pub trait IntoResponse {
    fn into_response(self) -> Response<Body>;
}

pub trait IntoResponseParts {
    type Error: IntoResponse;

    fn into_response_parts(self, res: ResponseParts) -> Result<ResponseParts, Self::Error>;
}

impl IntoResponse for Infallible {
    fn into_response(self) -> Response<Body> {
        match self {}
    }
}

macro_rules! impl_into_response {
    ($($ty:ident),* $(,)?) => {
        #[allow(non_snake_case, unused_parens)]
        impl<R, $($ty,)*> IntoResponse for ($($ty),*, R)
        where
            $($ty: IntoResponseParts,)*
            R: IntoResponse,
        {
            fn into_response(self) -> Response<Body> {
                let ($($ty),*, res) = self;

                let res = res.into_response();
                let (parts, body) = res.into_parts();

                $(
                    let parts = match $ty.into_response_parts(parts) {
                        Ok(parts) => parts,
                        Err(err) => {
                            return err.into_response();
                        }
                    };
                )*

                Response::from_parts(parts, body)
            }
        }
    };
}

all_the_tuples_no_last_special_case!(impl_into_response);

impl IntoResponseParts for StatusCode {
    type Error = Infallible;

    fn into_response_parts(self, mut res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        res.status = self;
        Ok(res)
    }
}

impl IntoResponse for StatusCode {
    fn into_response(self) -> Response<Body> {
        Response::builder()
            .status(self)
            .body(Body::empty())
            .unwrap()
    }
}

impl IntoResponse for Response<Body> {
    fn into_response(self) -> Response<Body> {
        self
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Response<Body> {
        let mut res = Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(self))
            .unwrap();
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain;charset=utf-8"),
        );
        res
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Response<Body> {
        let mut res = Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(self))
            .unwrap();
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain;charset=utf-8"),
        );
        res
    }
}

macro_rules! impl_from_request {
    ([$($ty:ident),*], $last:ident) => {
        #[allow(non_snake_case, unused_mut)]
        impl<$($ty,)* $last> FromRequest for ($($ty,)* $last,)
        where
            $($ty: FromRequestPart,)*
            $last: FromRequest,
        {
            type Rejection = Response<Body>;

            fn from_request(req: Request<Body>) -> Result<Self, Self::Rejection> {
                let (mut parts, body) = req.into_parts();

                $(
                    let $ty = $ty::from_request_parts(&mut parts).map_err(|err| err.into_response())?;
                )*

                let req = Request::from_parts(parts, body);

                let $last = $last::from_request(req).map_err(|err| err.into_response())?;

                Ok(($($ty,)* $last,))
            }
        }
    };
}

all_the_tuples!(impl_from_request);

pub trait Handler<T>: Clone + Send + Sized + 'static {
    fn call(self, req: Request<Body>) -> Response<Body>;
}

#[derive(Clone)]
pub struct HandlerService<H, T> {
    handler: H,
    extractors: PhantomData<T>,
}

impl<H, T> HandlerService<H, T> {
    pub fn new(handler: H) -> Self {
        Self {
            handler,
            extractors: Default::default(),
        }
    }
}

impl<H, T> Service for HandlerService<H, T>
where
    H: Handler<T>,
{
    type Body = Body;
    type Error = Infallible;

    fn call(&self, request: Request<Body>) -> Result<Response<Self::Body>, Self::Error> {
        Ok(self.handler.clone().call(request))
    }
}

macro_rules! impl_handler {
    ([$($ty:ident),*], $last:ident) => {
        #[allow(non_snake_case, unused_mut)]
        impl<F, $($ty,)* $last, Res> Handler<($($ty,)* $last,)> for F
        where
            F: FnOnce($($ty,)* $last,) -> Res + Clone + Send + 'static,
            $($ty: FromRequestPart,)*
            $last: FromRequest,
            Res: IntoResponse,
        {
            fn call(self, req: Request<Body>) -> Response<Body> {
                let (mut parts, body) = req.into_parts();

                $(
                    let $ty = match $ty::from_request_parts(&mut parts) {
                        Ok(val) => val,
                        Err(rejection) => return rejection.into_response(),
                    };
                )*

                let req = Request::from_parts(parts, body);

                let $last = match $last::from_request(req) {
                    Ok(val) => val,
                    Err(rejection) => return rejection.into_response(),
                };

                self($($ty,)* $last,).into_response()
            }
        }
    };
}

all_the_tuples!(impl_handler);

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

pub struct Route {
    svc: Box<dyn RoutedService<Body = Body, Error = Infallible>>,
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
    T: 'static + Send + Sync + Clone,
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
    type Error = Infallible;

    fn call(&self, request: Request<Body>) -> Result<Response<Self::Body>, Self::Error> {
        let path = request.uri().path();
        match self.router.at(path) {
            Ok(route) => route.value.clone().svc.call(request),
            Err(_) => Ok(StatusCode::NOT_FOUND.into_response()),
        }
    }
}
