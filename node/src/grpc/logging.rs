//! Tower middleware for gRPC access logging.
//!
//! Every RPC call is logged with method, status code, and latency.
//! Three log levels are used:
//!
//! - `INFO`  — every completed request (access log)
//! - `WARN`  — requests exceeding [`SLOW_THRESHOLD_MS`] ms
//! - `ERROR` — server-side errors (Internal, DataLoss, Unknown)
//!
//! # Note on status codes
//! The `grpc-status` header is only present on error responses at the HTTP
//! layer. Successful unary responses carry it in HTTP/2 trailers, which are
//! not visible without consuming the body. We therefore log `OK` for any
//! response without a `grpc-status` header — this is correct for all success
//! paths and means slow-request detection works even for successful RPCs.

use http::{HeaderMap, Request, Response};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Instant;
use tower::{Layer, Service};
use tracing::{error, info, warn};

/// Requests taking longer than this are logged at `WARN` level in addition
/// to the normal `INFO` access log entry.
const SLOW_THRESHOLD_MS: u128 = 5_000;

// ── Layer ─────────────────────────────────────────────────────────────────────

/// Tower [`Layer`] that wraps every gRPC service with access logging.
///
/// Apply to a tonic server via `Server::builder().layer(GrpcLoggingLayer)`.
#[derive(Clone, Default)]
pub struct GrpcLoggingLayer;

impl<S> Layer<S> for GrpcLoggingLayer {
    type Service = GrpcLoggingService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        GrpcLoggingService { inner }
    }
}

// ── Service ───────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct GrpcLoggingService<S> {
    inner: S,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for GrpcLoggingService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: std::fmt::Display,
    ReqBody: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let start = Instant::now();

        // `/package.Service/Method` — stable across retries and redirects.
        let method = req.uri().path().to_string();

        // Inner is guaranteed ready by the Tower contract (poll_ready → call).
        let future = self.inner.call(req);

        Box::pin(async move {
            let result = future.await;
            let latency_ms = start.elapsed().as_millis();

            match &result {
                Ok(response) => {
                    let code = status_from_headers(response.headers());
                    let status = format!("{code:?}");

                    // Access log — every request.
                    info!(method = %method, status = %status, latency_ms = latency_ms);

                    // Slow request warning.
                    if latency_ms >= SLOW_THRESHOLD_MS {
                        warn!(
                            method = %method,
                            status = %status,
                            latency_ms = latency_ms,
                            "slow gRPC request"
                        );
                    }

                    // Server error — something we need to investigate.
                    if is_server_error(code) {
                        error!(
                            method = %method,
                            status = %status,
                            latency_ms = latency_ms,
                            "gRPC server error"
                        );
                    }
                }
                Err(e) => {
                    error!(
                        method = %method,
                        latency_ms = latency_ms,
                        error = %e,
                        "gRPC transport error"
                    );
                }
            }

            result
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extract the gRPC status code from HTTP response headers.
/// Returns `Ok` when the header is absent (success path — status is in trailers).
fn status_from_headers(headers: &HeaderMap) -> tonic::Code {
    headers
        .get("grpc-status")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<i32>().ok())
        .map(tonic::Code::from)
        .unwrap_or(tonic::Code::Ok)
}

/// Returns `true` for codes that indicate a server-side fault rather than a
/// client error or expected condition.
fn is_server_error(code: tonic::Code) -> bool {
    matches!(code, tonic::Code::Internal | tonic::Code::DataLoss | tonic::Code::Unknown)
}
