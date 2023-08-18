use http::{Method, Request, StatusCode};
use spike::{HandlerService, IntoResponse};
use touche::{Body, Server, server::Service};

fn main() -> std::io::Result<()> {
    let handler = HandlerService::new(hello_world);

    Server::bind("0.0.0.0:4444").serve(move |req: Request<Body>| {
        let handler = handler.clone();
        handler.call(req)
    })
}

fn hello_world(method: Method, body: String) -> impl IntoResponse {
    (StatusCode::CREATED, format!("{method} - {body}"))
}
