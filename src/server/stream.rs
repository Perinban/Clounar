use axum::{
    body::Body,
    http::{header, StatusCode},
    response::Response,
};
use std::convert::Infallible;

use futures::StreamExt;
use tokio::sync::mpsc::Receiver;
use tokio_stream::wrappers::ReceiverStream;

pub fn stream_response(rx: Receiver<String>) -> Response {
    let body = Body::from_stream(ReceiverStream::new(rx).map(Ok::<_, Infallible>));
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONNECTION, "keep-alive")
        .body(body)
        .unwrap()
}
