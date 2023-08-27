use std::{convert::Infallible, marker::PhantomData};

use touche::{server::Service, Body, Request, Response};

use crate::{
    extract::{FromRequest, FromRequestPart},
    response::IntoResponse,
};

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

impl<F, Res> Handler<()> for F
where
    F: FnOnce() -> Res + Clone + Send + 'static,
    Res: IntoResponse,
{
    fn call(self, _req: Request<Body>) -> Response<Body> {
        self().into_response()
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
