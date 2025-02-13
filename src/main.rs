use clap::Parser;
use http_proxy::{admin::service, lb::LB};
use log::info;
use pingora::{
    prelude::{background_service, Opt},
    proxy::http_proxy_service,
    server::Server,
};

fn main() {
    env_logger::init();

    let opt = Opt::parse();
    let mut my_server = Server::new(Some(opt)).unwrap();
    my_server.bootstrap();

    let (mut admin_svc, resolver) = service().unwrap();
    info!("add admin http service service at 0.0.0.0:6100");
    admin_svc.add_tcp("0.0.0.0:6100");
    my_server.add_service(admin_svc);

    let backgrounds = resolver.backgrounds();
    let mut lb = http_proxy_service(&my_server.configuration, LB { backgrounds });
    info!("add http proxy service at 0.0.0.0:6188");
    lb.add_tcp("0.0.0.0:6188");
    my_server.add_service(lb);

    let resolver_bg_svc = background_service("resolver", resolver);
    my_server.add_service(resolver_bg_svc);

    info!("start server");
    my_server.run_forever();
}
