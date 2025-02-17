use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};

use crate::svcs::{Op, UpstreamsHealthCheck};

#[derive(Clone)]
pub struct RouteState {
    add_domain_queen: broadcast::Sender<Op>,
    backgrounds: Arc<RwLock<HashMap<String, Arc<UpstreamsHealthCheck>>>>,
}

impl RouteState {
    pub fn new(
        add_domain_queen: broadcast::Sender<Op>,
        backgrounds: Arc<RwLock<HashMap<String, Arc<UpstreamsHealthCheck>>>>,
    ) -> Self {
        Self {
            add_domain_queen,
            backgrounds,
        }
    }
}

pub fn routes(state: RouteState) -> Router {
    axum::Router::new()
        .route("/", get(hello))
        .route(
            "/domain",
            post(add_domain).delete(del_domain).get(get_domains),
        )
        .with_state(state)
}

async fn hello() -> &'static str {
    "Hello from Axum service in Pingora!"
}

#[derive(Debug, Deserialize, Serialize)]
struct ParamsDomain {
    domain: String,
}

async fn add_domain(
    State(state): State<RouteState>,
    Json(param): Json<ParamsDomain>,
) -> &'static str {
    state.add_domain_queen.send(Op::Add(param.domain)).unwrap();
    "ok"
}

async fn del_domain(
    State(state): State<RouteState>,
    Json(param): Json<ParamsDomain>,
) -> &'static str {
    state.add_domain_queen.send(Op::Del(param.domain)).unwrap();
    "ok"
}

#[derive(Debug, Deserialize, Serialize)]
struct DomainAddress {
    domain: String,
    address: Vec<String>,
}

async fn get_domains(State(state): State<RouteState>) -> (StatusCode, Json<Vec<DomainAddress>>) {
    let mut domains = Vec::new();
    for (domain, background) in state.backgrounds.read().await.iter() {
        domains.push(DomainAddress {
            domain: domain.clone(),
            address: background.get_backends(),
        });
    }
    (StatusCode::OK, Json(domains))
}
