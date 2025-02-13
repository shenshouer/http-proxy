use std::{collections::HashMap, ops::Deref, sync::Arc, time::Duration};

use async_trait::async_trait;
use axum::Router;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{header, StatusCode};
use log::info;
use pingora::{apps::http_app::ServeHttp, prelude::timeout, protocols::http::ServerSession};
use tokio::sync::{broadcast, RwLock};
use tower::ServiceExt;

use crate::svcs::{self, DNSResolver, Op, UpstreamsHealthCheck};

use super::route::{routes, RouteState};

pub struct HttpAdminApp {
    routes: Router,
    add_domain_queen: broadcast::Receiver<Op>,
    backgrounds: Arc<RwLock<HashMap<String, Arc<UpstreamsHealthCheck>>>>,
}

impl Default for HttpAdminApp {
    fn default() -> Self {
        let backgrounds = Arc::new(RwLock::new(HashMap::new()));
        let (tx, add_domain_queen) = broadcast::channel(5);
        let state = RouteState::new(tx, backgrounds.clone());
        let routes = routes(state);
        Self {
            routes,
            add_domain_queen,
            backgrounds,
        }
    }
}

impl HttpAdminApp {
    pub fn dns_resolver(&self) -> Result<DNSResolver, svcs::Error> {
        let resolver = DNSResolver::new(
            None,
            None,
            self.add_domain_queen.resubscribe(),
            self.backgrounds.clone(),
        )?;
        Ok(resolver)
    }
}

#[async_trait]
impl ServeHttp for HttpAdminApp {
    async fn response(&self, http_stream: &mut ServerSession) -> hyper::Response<Vec<u8>> {
        info!("HttpAdminApp::response called");
        match http_stream.to_request().await {
            Err(res) => res,
            Ok(req) => {
                let (mut parts, body) = axum::response::IntoResponse::into_response(
                    self.routes.clone().oneshot(req).await,
                )
                .into_parts();
                // TODO: 此处流式数据处理可能会存在问题
                let bytes = BodyExt::collect(body).await.unwrap().to_bytes();
                parts
                    .headers
                    .insert(header::CONTENT_LENGTH, bytes.len().into());
                hyper::Response::from_parts(parts, bytes.to_vec())
            }
        }
    }
}

trait ToRequest {
    async fn to_request(&mut self)
        -> Result<hyper::Request<Full<Bytes>>, hyper::Response<Vec<u8>>>;
}

impl ToRequest for ServerSession {
    async fn to_request(
        &mut self,
    ) -> Result<hyper::Request<Full<Bytes>>, hyper::Response<Vec<u8>>> {
        let read_timeout = 2000;
        let body = match timeout(
            Duration::from_millis(read_timeout),
            self.read_request_body(),
        )
        .await
        {
            Ok(res) => match res.unwrap() {
                Some(bytes) => Full::new(bytes),
                None => Full::new(Bytes::new()),
            },
            Err(_) => {
                return Err((
                    StatusCode::REQUEST_TIMEOUT,
                    format!("Timed out after {:?}ms", read_timeout),
                )
                    .into_response());
            }
        };

        let headers = self.req_header();
        let parts = headers.deref().to_owned();
        Ok(hyper::Request::from_parts(parts, body))
    }
}

pub trait IntoResponse<T> {
    #[must_use]
    fn into_response(self) -> hyper::Response<T>;
}

impl IntoResponse<Vec<u8>> for (StatusCode, &str) {
    fn into_response(self) -> hyper::Response<Vec<u8>> {
        let body = self.1.as_bytes().to_vec();
        hyper::Response::builder()
            .status(self.0)
            .header(header::CONTENT_TYPE, "text/html")
            .header(header::CONTENT_LENGTH, body.len())
            .body(body)
            .unwrap()
    }
}

impl IntoResponse<Vec<u8>> for (StatusCode, String) {
    fn into_response(self) -> hyper::Response<Vec<u8>> {
        let body = self.1.as_bytes().to_vec();
        hyper::Response::builder()
            .status(self.0)
            .header(header::CONTENT_TYPE, "text/html")
            .header(header::CONTENT_LENGTH, body.len())
            .body(body)
            .unwrap()
    }
}
