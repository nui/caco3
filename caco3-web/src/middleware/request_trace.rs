use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::extract::MatchedPath;
use axum::http::{Method, Request, Response, Uri};
use futures_core::ready;
use pin_project::pin_project;
use tower::{Layer, Service};
use tracing::trace;

pub trait RequestTrace {
    fn is_traced(&self, path: Option<&MatchedPath>) -> bool;

    fn enabled(&self) -> bool {
        true
    }
}

/// A struct contain Http request info.
#[derive(Debug, Clone)]
pub struct RequestTraceData {
    /// Indicate that request trace should be shown for route.
    pub trace: bool,
    /// Request method.
    pub method: Method,
    /// Request uri.
    pub uri: Uri,
}

#[derive(Debug, Clone)]
/// Middleware that adds [`RequestTraceData`] to response extension.
pub struct RequestTraceService<S, F> {
    inner: S,
    make_tracer: F,
}

#[derive(Clone)]
/// [`Layer`] that adds [`RequestTraceData`] to response extension.
pub struct RequestTraceLayer<F> {
    make_tracer: F,
}

impl<S, F> Layer<S> for RequestTraceLayer<F>
where
    F: Clone,
{
    type Service = RequestTraceService<S, F>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestTraceService {
            inner,
            make_tracer: self.make_tracer.clone(),
        }
    }
}

impl<F> RequestTraceLayer<F> {
    pub fn new(make_tracer: F) -> Self {
        Self { make_tracer }
    }
}

impl<ReqBody, ResBody, S, F, T> Service<Request<ReqBody>> for RequestTraceService<S, F>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
    F: FnMut() -> T,
    T: RequestTrace,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = RequestTraceFuture<Request<ReqBody>, S>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let tracer = (self.make_tracer)();
        let enabled = tracer.enabled();
        let mut request_trace = None;
        let mut matched_path = None;

        if enabled {
            matched_path = req.extensions().get::<MatchedPath>();
            let trace = tracer.is_traced(matched_path);
            request_trace = Some(RequestTraceData {
                trace,
                method: req.method().clone(),
                uri: req.uri().clone(),
            });
        }

        trace!(
            "RequestTraceService: enabled = {enabled}, matched_path = {matched_path:?}, \
            request_trace = {request_trace:?}",
        );

        RequestTraceFuture {
            request_trace,
            state: FutureState::Polling(self.inner.call(req)),
        }
    }
}

#[pin_project]
pub struct RequestTraceFuture<Request, S: Service<Request>> {
    request_trace: Option<RequestTraceData>,
    #[pin]
    state: FutureState<Request, S>,
}

#[pin_project(project = FutureStateProj)]
enum FutureState<Request, S: Service<Request>> {
    Polling(#[pin] S::Future),
    Finished,
}

impl<Request, ResBody, S> Future for RequestTraceFuture<Request, S>
where
    S: Service<Request, Response = Response<ResBody>>,
{
    type Output = Result<S::Response, S::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        match this.state.as_mut().project() {
            FutureStateProj::Polling(service_fut) => {
                let mut output: Self::Output = ready!(service_fut.poll(cx));
                // TODO: Decide if we should set finished state earlier if panic may occur
                if let Ok(response) = &mut output {
                    if let Some(request_trace) = this.request_trace.take() {
                        response.extensions_mut().insert(request_trace);
                    }
                }
                this.state.set(FutureState::Finished);
                Poll::Ready(output)
            }
            FutureStateProj::Finished => {
                panic!("RequestTraceFuture polled after completion");
            }
        }
    }
}
