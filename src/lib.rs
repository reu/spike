use std::{convert::Infallible, io, marker::PhantomData, str::Utf8Error};

use http::{
    request::Parts as RequestParts, response::Parts as ResponseParts, HeaderMap, HeaderValue,
};
use touche::{header, server::Service, Body, HttpBody, Method, Request, Response, StatusCode};

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

impl<A, B> IntoResponse for (A, B)
where
    A: IntoResponseParts,
    B: IntoResponse,
{
    fn into_response(self) -> Response<Body> {
        let (a, b) = self;
        let res = b.into_response();
        let (parts, body) = res.into_parts();

        let parts = match a.into_response_parts(parts) {
            Ok(parts) => parts,
            Err(err) => return err.into_response(),
        };

        Response::from_parts(parts, body)
    }
}

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

impl<A, B> FromRequest for (A, B)
where
    A: FromRequestPart,
    B: FromRequest,
{
    type Rejection = Response<Body>;

    fn from_request(req: Request<Body>) -> Result<Self, Self::Rejection> {
        let (mut parts, body) = req.into_parts();

        let first = A::from_request_parts(&mut parts).map_err(|err| err.into_response())?;

        let req = Request::from_parts(parts, body);

        let last = B::from_request(req).map_err(|err| err.into_response())?;

        Ok((first, last))
    }
}

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

impl<F, T1, T2, Res> Handler<(T1, T2)> for F
where
    F: FnOnce(T1, T2) -> Res + Clone + Send + 'static,
    T1: FromRequestPart,
    T2: FromRequest,
    Res: IntoResponse,
{
    fn call(self, req: Request<Body>) -> Response<Body> {
        let (mut parts, body) = req.into_parts();
        let t1 = match T1::from_request_parts(&mut parts) {
            Ok(val) => val,
            Err(rejection) => return rejection.into_response(),
        };

        let req = Request::from_parts(parts, body);

        let last = match T2::from_request(req) {
            Ok(val) => val,
            Err(rejection) => return rejection.into_response(),
        };

        self(t1, last).into_response()
    }
}
