//! Logging module
use std::time::SystemTime;

use tracing::{Instrument, Level};

use salvo_core::async_trait;
use salvo_core::http::{Request, Response, StatusCode};
use salvo_core::routing::FlowCtrl;
use salvo_core::{Depot, Handler};

/// LogHandler
#[derive(Default, Debug)]
pub struct LogHandler;

#[async_trait]
impl Handler for LogHandler {
    async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
        let span = tracing::span!(
            Level::INFO,
            "Request",
            remote_addr = %req.remote_addr().map(|addr|addr.to_string()).unwrap_or_else(|| "[Unknown]".into()),
            version = ?req.version(),
            method = %req.method(),
            path = %req.uri(),
        );

        async move {
            let now = SystemTime::now();
            ctrl.call_next(req, depot, res).await;

            let status = match res.status_code() {
                Some(code) => code,
                None => {
                    if res.body().is_none() {
                        StatusCode::NOT_FOUND
                    } else {
                        StatusCode::OK
                    }
                }
            };
            match now.elapsed() {
                Ok(duration) => {
                    tracing::info!(
                        status = %status,
                        duration = ?duration,
                        "Response"
                    );
                }
                Err(_) => {
                    tracing::info!(
                        status = %status,
                        "Response"
                    );
                }
            }
        }
        .instrument(span)
        .await
    }
}

#[cfg(test)]
mod tests {
    use salvo_core::prelude::*;
    use tracing_test::traced_test;

    use super::*;

    #[tokio::test]
    #[traced_test]
    async fn test_log() {
        #[fn_handler]
        async fn hello() -> &'static str {
            "hello"
        }

        let router = Router::new()
            .hoop(LogHandler)
            .push(Router::with_path("hello").get(hello));
        let service = Service::new(router);

        let req: Request = hyper::Request::builder()
            .method("GET")
            .uri("http://127.0.0.1:7979/hello")
            .body(hyper::Body::empty())
            .unwrap()
            .into();
        service.handle(req).await.take_text().await.unwrap();
        assert!(logs_contain("duration"));
    }
}