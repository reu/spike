use std::{borrow::Cow, convert::Infallible};

use touche::{
    header,
    http::{response::Parts as ResponseParts, HeaderValue},
    Body, Response, StatusCode,
};

pub trait IntoResponse {
    fn into_response(self) -> Response<Body>;
}

pub trait IntoResponseParts {
    type Error: IntoResponse;

    fn into_response_parts(self, res: ResponseParts) -> Result<ResponseParts, Self::Error>;
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

impl IntoResponse for Infallible {
    fn into_response(self) -> Response<Body> {
        match self {}
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

impl IntoResponse for &'static [u8] {
    fn into_response(self) -> Response<Body> {
        Cow::Borrowed(self).into_response()
    }
}

impl<const N: usize> IntoResponse for &'static [u8; N] {
    fn into_response(self) -> Response<Body> {
        self.as_slice().into_response()
    }
}

impl<const N: usize> IntoResponse for [u8; N] {
    fn into_response(self) -> Response<Body> {
        self.to_vec().into_response()
    }
}

impl IntoResponse for Vec<u8> {
    fn into_response(self) -> Response<Body> {
        Cow::<'static, [u8]>::Owned(self).into_response()
    }
}

impl IntoResponse for Cow<'static, [u8]> {
    fn into_response(self) -> Response<Body> {
        let mut res = Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(self.as_ref()))
            .unwrap();
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );
        res
    }
}
