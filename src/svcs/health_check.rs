use std::{net::SocketAddr, sync::Arc, time::Duration};

use async_trait::async_trait;
use pingora::{
    lb::{health_check, LoadBalancer},
    prelude::{background_service, RoundRobin},
    server::ShutdownWatch,
    services::background::{BackgroundService, GenBackgroundService},
};
use pingora_runtime::current_handle;
use tokio::{sync::watch, time::interval};

pub struct UpstreamsHealthCheck {
    stop_sender: watch::Sender<bool>,
    upstreams: Arc<GenBackgroundService<LoadBalancer<RoundRobin>>>,
}

impl UpstreamsHealthCheck {
    pub fn new(upstreams: GenBackgroundService<LoadBalancer<RoundRobin>>) -> Self {
        let (stop_sender, _) = watch::channel(false);
        Self {
            stop_sender,
            upstreams: Arc::new(upstreams),
        }
    }

    pub fn task(&self) -> Arc<LoadBalancer<RoundRobin>> {
        self.upstreams.task()
    }

    pub fn stop(&self) {
        let _ = self.stop_sender.send(true);
    }

    pub fn get_backends(&self) -> Vec<String> {
        let ups = self.upstreams.task();
        ups.backends()
            .get_backend()
            .iter()
            .map(|b| b.to_string())
            .collect()
    }
}

impl From<Vec<SocketAddr>> for UpstreamsHealthCheck {
    fn from(socket_addr: Vec<SocketAddr>) -> Self {
        let mut upstreams: LoadBalancer<RoundRobin> =
            LoadBalancer::try_from_iter(socket_addr).unwrap();

        let hc = health_check::TcpHealthCheck::new();
        upstreams.set_health_check(hc);
        upstreams.health_check_frequency = Some(Duration::from_secs(1));
        let background = background_service("health check", upstreams);

        Self::new(background)
    }
}

#[async_trait]
impl BackgroundService for UpstreamsHealthCheck {
    async fn start(&self, mut shutdown: ShutdownWatch) {
        let mut period = interval(Duration::from_secs(1));
        let mut stop_receiver = self.stop_sender.subscribe();

        let stop_receiver_clone = stop_receiver.clone();
        let upstreams_clone = self.upstreams.clone();
        current_handle().spawn(async move {
            upstreams_clone.task().start(stop_receiver_clone).await;
        });
        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    println!("Shutdown.");
                    break;
                }
                _ = stop_receiver.changed() => {
                    println!("Received stop signal.");
                    break;
                }
                _ = period.tick() => {}
            }
        }
    }
}
