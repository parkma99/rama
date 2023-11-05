use std::time::Duration;

use rama::{
    graceful::Shutdown, server::tcp::TcpListener, service::limit::ConcurrentPolicy,
    stream::service::EchoService,
};

use tracing::metadata::LevelFilter;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy(),
        )
        .init();

    let shutdown = Shutdown::default();

    shutdown.spawn_task_fn(|guard| async {
        let tcp_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind TCP Listener");
        tracing::info!(
            "listening for incoming TCP connections on {}",
            tcp_listener.local_addr().unwrap()
        );

        tcp_listener.set_ttl(30).expect("set TTL");

        tcp_listener
            .spawn()
            .limit(ConcurrentPolicy::new(2))
            .timeout(Duration::from_secs(30))
            .serve_graceful::<_, EchoService, _>(guard, EchoService::new())
            .await
            .expect("serve incoming TCP connections");
    });

    shutdown
        .shutdown_with_limit(Duration::from_secs(30))
        .await
        .expect("graceful shutdown");
}
