use std::net::ToSocketAddrs;

#[tokio::main]
async fn main() {
    let domain = "www.baidu.com";
    println!(
        "-->get_domain_ips1 {:?}",
        get_domain_ips1(&format!("{domain}:80"))
    );
    println!(
        "-->get_domain_ips2 {:?}",
        get_domain_ips2(&format!("{domain}:80")).await
    );
    println!("-->get_domain_ips3 {:?}", get_domain_ips3(domain).await);
    println!("-->get_domain_ips4 {:?}", get_domain_ips4(domain).await);
    println!("-->get_domain_ips5 {:?}", get_domain_ips5(domain).await);
}

/// 使用标准库
fn get_domain_ips1(domain: &str) -> Result<Vec<std::net::IpAddr>, std::io::Error> {
    domain
        .to_socket_addrs()
        .map(|iter| iter.map(|socket_addr| socket_addr.ip()).collect())
}

/// 使用tokio
async fn get_domain_ips2(domain: &str) -> Result<Vec<std::net::IpAddr>, std::io::Error> {
    let x = tokio::net::lookup_host(domain).await;
    x.map(|iter| iter.map(|socket_addr| socket_addr.ip()).collect())
}

/// 使用hickory-resolver
async fn get_domain_ips3(domain: &str) -> Result<Vec<std::net::IpAddr>, std::io::Error> {
    use hickory_resolver::{system_conf, TokioAsyncResolver};

    // let resolver = Resolver::new(ResolverConfig::default(), ResolverOpts::default())?;
    // 默认使用google的dns服务器
    // let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());
    let (config, opts) = system_conf::read_system_conf()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    let resolver = TokioAsyncResolver::tokio(config, opts);

    let response = resolver
        .lookup_ip(domain)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    Ok(response.iter().collect())
}

/// 使用hickory-resolver cloudflare dns
async fn get_domain_ips4(domain: &str) -> Result<Vec<std::net::IpAddr>, std::io::Error> {
    use hickory_resolver::{
        config::{NameServerConfig, Protocol, ResolverConfig, ResolverOpts},
        error::ResolveError,
        TokioAsyncResolver,
    };
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    let mut opts = ResolverOpts::default();
    opts.timeout = std::time::Duration::from_secs(5);

    // 创建自定义配置使用 Cloudflare DNS (1.1.1.1)
    let mut config = ResolverConfig::new();
    config.add_name_server(NameServerConfig::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 53),
        Protocol::Udp,
    ));

    // 可选：添加备用 DNS (1.0.0.1)
    config.add_name_server(NameServerConfig::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 0, 0, 1)), 53),
        Protocol::Udp,
    ));

    let resolver = TokioAsyncResolver::tokio(config, opts);

    let response = resolver
        .lookup_ip(domain)
        .await
        .map_err(|e: ResolveError| {
            eprintln!("DNS resolution error: {:?}", e);
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("DNS resolution failed: {}", e),
            )
        })?;

    Ok(response.iter().collect())
}

/// 使用Cloudflare DoH
/// curl -s -H "accept: application/dns-json" "https://cloudflare-dns.com/dns-query?name=www.baidu.com&type=A" | jq .
async fn get_domain_ips5(domain: &str) -> Result<Vec<std::net::IpAddr>, std::io::Error> {
    use hickory_resolver::{
        config::{ResolverConfig, ResolverOpts},
        error::ResolveError,
        TokioAsyncResolver,
    };
    let mut opts = ResolverOpts::default();
    opts.timeout = std::time::Duration::from_secs(5); // 设置5秒超时

    let resolver = TokioAsyncResolver::tokio(
        // ResolverConfig::cloudflare_https(), // 直接使用 Cloudflare 的默认 DoH 配置
        ResolverConfig::cloudflare_tls(),
        opts,
    );

    let response = resolver
        .lookup_ip(domain)
        .await
        .map_err(|e: ResolveError| {
            eprintln!("DNS resolution error: {:?}", e); // 打印详细错误信息
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("DNS resolution failed: {}", e),
            )
        })?;

    Ok(response.iter().collect())
}
