use http::{Method, StatusCode};
use spike::{get, post, put, IntoResponse, Router};
use touche::Server;

fn main() -> std::io::Result<()> {
    let router = Router::new()
        .route("/hello", get(hello_world).post(hello_post))
        .route("/hello", put(put_hello_world).any(any_hello))
        .route("/jesus", post(jesus))
        .route("/world", get(world));

    Server::bind("0.0.0.0:4444").serve(router)
}

fn hello_world(method: Method, body: String) -> impl IntoResponse {
    (StatusCode::CREATED, format!("Hello: {method} - {body}"))
}

fn put_hello_world(method: Method, body: String) -> impl IntoResponse {
    (StatusCode::CREATED, format!("Hello: {method} - {body}"))
}

fn hello_post(method: Method, body: String) -> impl IntoResponse {
    (StatusCode::CREATED, format!("Hello: {method} - {body}"))
}

fn any_hello(method: Method, body: String) -> impl IntoResponse {
    (StatusCode::CREATED, format!("Any Hello: {method} - {body}"))
}

fn world(method: Method, body: String) -> impl IntoResponse {
    (StatusCode::CREATED, format!("World: {method} - {body}"))
}

fn jesus(method: Method) -> impl IntoResponse {
    format!("Jesus {method}")
}
