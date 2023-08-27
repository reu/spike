use spike::{
    http::{Method, StatusCode},
    response::IntoResponse,
    routing::{get, put},
    Router, Server,
};

fn main() -> std::io::Result<()> {
    let router = Router::new()
        .route("/hello", get(hello_world).post(hello_post))
        .route("/hello", put(put_hello_world).any(any_hello))
        .route("/hi", get(|| "Hi world"))
        .route("/world", get(world));

    Server::bind("0.0.0.0:4444").serve(router)
}

fn hello_world(method: Method, body: String) -> impl IntoResponse {
    (StatusCode::OK, format!("Hello: {method} - {body}"))
}

fn put_hello_world(method: Method, body: String) -> impl IntoResponse {
    (StatusCode::CREATED, format!("Hello: {method} - {body}"))
}

fn hello_post(body: String) -> impl IntoResponse {
    (StatusCode::CREATED, format!("POST Hello: {body}"))
}

fn any_hello(method: Method, body: String) -> impl IntoResponse {
    (StatusCode::CREATED, format!("Any Hello: {method} - {body}"))
}

fn world() -> impl IntoResponse {
    (StatusCode::OK, format!("World"))
}
