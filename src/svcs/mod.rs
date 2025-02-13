use std::fmt;

use thiserror::Error;

pub use dns_resolver::DNSResolver;
pub use health_check::UpstreamsHealthCheck;

mod dns_resolver;
mod health_check;

#[derive(Debug, Error)]
pub enum Error {
    #[error("DNS resolution error: {0}")]
    Resolver(#[from] anyhow::Error),
}

#[derive(Clone)]
pub enum Op {
    Add(String),
    Del(String),
}

impl fmt::Debug for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Op::Add(domain) => write!(f, "Add domain: {domain}"),
            Op::Del(domain) => write!(f, "Remove domain: {domain}"),
        }
    }
}
