use http::{Method, StatusCode};
use spike::{router::on, IntoResponse, Router};
use touche::Server;

fn main() -> std::io::Result<()> {
    let router = Router::new()
        .route("/hello", on(hello_world))
        .route("/jesus", on(jesus))
        .route("/world", on(world));

    Server::bind("0.0.0.0:4444").serve(router)
}

fn hello_world(method: Method, body: String) -> impl IntoResponse {
    (StatusCode::CREATED, format!("Hello: {method} - {body}"))
}

fn world(method: Method, body: String) -> impl IntoResponse {
    (StatusCode::CREATED, format!("World: {method} - {body}"))
}

fn jesus(method: Method) -> impl IntoResponse {
    format!("Jesus {method}")
}
