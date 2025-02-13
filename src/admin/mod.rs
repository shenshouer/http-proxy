use app::HttpAdminApp;
use pingora::services::listening::Service;

use crate::svcs::{self, DNSResolver};

mod app;
mod route;

pub fn service() -> Result<(Service<HttpAdminApp>, DNSResolver), svcs::Error> {
    let app = HttpAdminApp::default();
    let resolver = app.dns_resolver()?;
    let svc = Service::new("Admin Service HTTP".to_string(), app);
    Ok((svc, resolver))
}
