use axum::response::Response;

use super::dispatcher::Dispatcher;

pub trait Respond {
    async fn stream(self, d: &Dispatcher) -> Response;
    async fn blocking(self, d: &Dispatcher) -> Response;
}
