use std::{env, error::Error, future::Future, net::Ipv6Addr, pin::Pin, time::Duration};

use bb8_bolt::{
    bb8,
    bolt_proto::version::{V4_2, V4_3},
};
use log::{info, warn};
use proto::user_service_server::UserServiceServer;
use tokio::signal::unix::{signal, SignalKind};
use tonic::{transport::Server, Response, Status};
use tonic_health::server::health_reporter;
use user_service::UserService;

pub mod proto;
mod user_service;

type AsyncWrapper<'a, T> = Pin<Box<dyn Future<Output = Result<Response<T>, Status>> + Send + 'a>>;

type DynError = Box<dyn Error + Send + Sync>;

/// This function will initialize the [env-logger](https://docs.rs/env_logger) and start the server.  
/// Because this function will be used in integration tests,
/// it will **NOT** block the main thread.
///
/// # Panics
///
/// Panics if called from **outside** of the Tokio runtime.
pub async fn start_up() -> Result<(), DynError> {
    env_logger::init();

    let bolt_metadata: bb8_bolt::bolt_client::Metadata = [
        ("user_agent", "MiniTikTok-User/0"),
        ("scheme", "basic"),
        (
            "principal",
            // TODO: String::leak
            Box::leak(
                env::var("BOLT_USERNAME")
                    .map_err(|_| "BOLT_USERNAME doesn't exist.")?
                    .into_boxed_str(),
            ),
        ),
        (
            "credentials",
            // TODO: String::leak
            Box::leak(
                env::var("BOLT_PASSWORD")
                    .map_err(|_| "BOLT_PASSWORD doesn't exist.")?
                    .into_boxed_str(),
            ),
        ),
    ]
    .into_iter()
    .collect();

    let bolt_url = env::var("BOLT_URL").map_err(|_| "BOLT_URL doesn't exist.")?;

    let bolt_domain = env::var("BOLT_DOMAIN").ok();

    let (mut health_reporter, health_service) = health_reporter();

    let bolt_manager =
        bb8_bolt::Manager::new(bolt_url, bolt_domain, [V4_3, V4_2, 0, 0], bolt_metadata).await?;

    health_reporter
        .set_serving::<UserServiceServer<UserService>>()
        .await;

    let mut sigterm = signal(SignalKind::terminate())?;

    Server::builder()
        .concurrency_limit_per_connection(256)
        .tcp_keepalive(Some(Duration::from_secs(10)))
        .add_service(health_service)
        .add_service(UserServiceServer::new(UserService {
            bolt_pool: bb8::Pool::builder().build(bolt_manager).await?,
        }))
        .serve_with_shutdown(
            (Ipv6Addr::UNSPECIFIED, 14514).into(),
            // TODO?: unwrap
            async {
                match sigterm.recv().await {
                    Some(()) => info!("start graceful shutdown"),
                    None => warn!("stream of SIGTERM closed"),
                }
            },
        )
        .await?;

    Ok(())
}

/// Build a runtime and block on a `Future`.
pub fn block_on<F: std::future::Future>(f: F) -> Result<F::Output, std::io::Error> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(f))
}
