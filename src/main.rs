#![deny(clippy::pedantic)]

use crate::endpoints::{board, submit};
use axum::routing::{get, post};
use axum::{serve, Router};
use std::env::var;
use std::error::Error;
use std::net::SocketAddr;
use tokio::net::TcpListener;

mod endpoints;
mod pixels;
mod requests;

#[tokio::main]
async fn main() -> Result<(), impl Error> {
    let app = Router::new()
        .route("/v1/board", get(board))
        .route("/v1/submit", post(submit))
        .into_make_service_with_connect_info::<SocketAddr>();

    let address = var("BIND_ADDR").unwrap_or_else(|_| "[::1]:8080".to_string());
    let listener = TcpListener::bind(address).await?;

    serve(listener, app).await
}
