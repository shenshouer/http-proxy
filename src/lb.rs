use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use bytes::Bytes;
use pingora::{
    prelude::HttpPeer,
    proxy::{ProxyHttp, Session},
    Error, ErrorSource,
    ErrorType::{self, *},
    Result,
};
use tokio::sync::RwLock;

use crate::svcs::UpstreamsHealthCheck;

pub struct LB {
    pub backgrounds: Arc<RwLock<HashMap<String, Arc<UpstreamsHealthCheck>>>>,
}

#[async_trait]
impl ProxyHttp for LB {
    type CTX = ();
    fn new_ctx(&self) -> Self::CTX {}

    async fn upstream_peer(&self, session: &mut Session, _ctx: &mut ()) -> Result<Box<HttpPeer>> {
        let headers = session.req_header();
        if let Some(domain) = headers.headers.get("host") {
            let domain = domain.to_str().unwrap();

            let backgrounds_lock = self.backgrounds.read().await;
            let upstreams = backgrounds_lock.get(domain).ok_or_else(|| {
                Error::new_str(
                    format!("Domain {domain} not found in backgrounds, Did you add it?").leak(),
                )
            })?;
            let upstream = upstreams.task().select(b"", 256).ok_or_else(|| {
                let mut err =
                    Error::new_str(format!("Select upstream failed when request {domain}").leak());
                err.as_in();
                err
            })?;
            let peer = Box::new(HttpPeer::new(upstream, true, domain.to_string()));
            return Ok(peer);
        }
        let mut err = Error::new_str("Host not found ");
        err.as_down();
        err.etype = ErrorType::InvalidHTTPHeader;
        Err(err)
    }

    async fn fail_to_proxy(&self, session: &mut Session, e: &Error, _ctx: &mut Self::CTX) -> u16
    where
        Self::CTX: Send + Sync,
    {
        let server_session = session.as_mut();
        let code = match e.etype() {
            HTTPStatus(code) => *code,
            _ => match e.esource() {
                ErrorSource::Upstream => 502,
                ErrorSource::Downstream => match e.etype() {
                    WriteError | ReadError | ConnectionClosed => 0,
                    _ => 400,
                },
                ErrorSource::Internal | ErrorSource::Unset => 500,
            },
        };
        if code > 0 {
            // current release 0.4.0 of pingora does not support respond_error_with_body
            // so depend on the main branch of pingora
            server_session
                .respond_error_with_body(code, Bytes::from(e.to_string()))
                .await
                .ok();
        }
        code
    }
}
