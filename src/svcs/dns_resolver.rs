use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use anyhow::Context;
use async_trait::async_trait;
use hickory_resolver::{
    config::{ResolverConfig, ResolverOpts},
    system_conf, TokioAsyncResolver,
};
use log::info;
use pingora::{server::ShutdownWatch, services::background::BackgroundService};
use pingora_runtime::current_handle;
use tokio::sync::{broadcast, RwLock};

use super::{Error, Op, UpstreamsHealthCheck};

/// DNS 解析器
/// 用于解析域名并将解析结果转换为 LoadBalancer
pub struct DNSResolver {
    resolver: TokioAsyncResolver,
    add_domain_queen: broadcast::Receiver<Op>,
    waitings_sender: broadcast::Sender<(String, Vec<SocketAddr>)>,
    waitings_receiver: broadcast::Receiver<(String, Vec<SocketAddr>)>,
    backgrounds: Arc<RwLock<HashMap<String, Arc<UpstreamsHealthCheck>>>>,
}

impl DNSResolver {
    pub fn new(
        config: Option<ResolverConfig>,
        options: Option<ResolverOpts>,
        add_domain_queen: broadcast::Receiver<Op>,
        backgrounds: Arc<RwLock<HashMap<String, Arc<UpstreamsHealthCheck>>>>,
    ) -> Result<Self, Error> {
        let (sys_config, sys_options) =
            system_conf::read_system_conf().context("DNS Resolver read system config failed")?;

        let (config, options) = (config.unwrap_or(sys_config), options.unwrap_or(sys_options));
        let resolver = TokioAsyncResolver::tokio(config, options);
        // 解析后的生成的 UpstreamsHealthCheck 服务队列
        let (waitings_sender, waitings_receiver) = broadcast::channel(5);
        Ok(Self {
            resolver,
            add_domain_queen,
            waitings_sender,
            waitings_receiver,
            backgrounds,
        })
    }

    /// 添加一个域名
    /// 会将域名解析为 IP 地址，并创建一个 UpstreamsHealthCheck 服务 提供默认的健康检查
    /// 并发送到 waitings 通道中，等待后台服务启动
    async fn add(&self, domain: &str) -> Result<(), Error> {
        info!("DNSResolver::add {domain}");
        let socket_addr = self
            .resolver
            .lookup_ip(domain)
            .await
            .context("Resolve domain {} failed")?
            .iter()
            .map(|ip| SocketAddr::new(ip, 443))
            .collect::<Vec<_>>();

        self.waitings_sender
            .send((domain.to_owned(), socket_addr))
            .context("Send to start background service failed")?;

        Ok(())
    }

    async fn remove(&self, domain: &str) {
        if let Some(background) = self.backgrounds.write().await.remove(domain) {
            background.stop();
        }
    }

    pub fn backgrounds(&self) -> Arc<RwLock<HashMap<String, Arc<UpstreamsHealthCheck>>>> {
        self.backgrounds.clone()
    }
}

#[async_trait]
impl BackgroundService for DNSResolver {
    async fn start(&self, mut shutdown: ShutdownWatch) {
        // let mut period = interval(Duration::from_secs(1));
        let mut receiver = self.waitings_receiver.resubscribe();
        let mut add_domain_queen = self.add_domain_queen.resubscribe();
        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    println!("DNSResolver Shutdown.");
                    let backgrounds = self.backgrounds.write().await;
                    for (_, background) in backgrounds.iter() {
                        background.stop();
                    }
                    break;
                }
                data = receiver.recv() => {
                    info!("BackgroundService/DNSResolver::start Received new domain task {data:?}.");
                    if let Ok((domain, socket_addr)) = data {
                        let background = Arc::new(UpstreamsHealthCheck::from(socket_addr));
                        // TODO: 此处是否需要spawn一个新的任务?
                        let background_clone = background.clone();
                        let shutdown = shutdown.clone();
                        current_handle().spawn(async move  {
                            background_clone.start(shutdown.clone()).await;
                        });
                        self.backgrounds.write().await.insert(domain.clone(), background);
                        println!("Received new domain task {domain}.");
                    }
                }
                op = add_domain_queen.recv() => {
                    if let Ok(op) = op {
                        match op {
                            Op::Add(domain) => {
                                self.add(&domain).await.unwrap();
                            }
                            Op::Del(domain) => {
                                self.remove(&domain).await;
                            }
                        }
                    }
                }
                // _ = period.tick() => {}
            }
        }
    }
}
