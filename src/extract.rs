use std::{convert::Infallible, io, str::Utf8Error};

use http::{request::Parts as RequestParts, HeaderMap};
use touche::{Body, HttpBody, Method, Request, Response, StatusCode};

use crate::response::IntoResponse;

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
