use std::{
    env,
    error::Error,
    future::Future,
    net::{Ipv4Addr, Ipv6Addr},
    pin::Pin,
    time::Duration,
};

use bb8_bolt::{
    bb8,
    bolt_proto::version::{V4_2, V4_3},
};
use combind_incoming::CombinedIncoming;
use health_check::HealthChecker;
use proto::{
    auth_service_client::AuthServiceClient, health_checker_server::HealthCheckerServer,
    user_service_server::UserServiceServer,
};
use tokio::task::JoinHandle;
use tonic::{transport::Server, Response, Status};
use user_service::UserService;

mod combind_incoming;
mod health_check;
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
pub fn start_up() -> Result<JoinHandle<Result<(), DynError>>, &'static str> {
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

    let auth_url = env::var("AUTH_URL").map_err(|_| "AUTH_URL doesn't exist.")?;

    Ok(tokio::spawn(async {
        let bolt_manager =
            bb8_bolt::Manager::new(bolt_url, bolt_domain, [V4_3, V4_2, 0, 0], bolt_metadata)
                .await?;

        Server::builder()
            .concurrency_limit_per_connection(256)
            .tcp_keepalive(Some(Duration::from_secs(10)))
            .add_service(HealthCheckerServer::new(HealthChecker))
            .add_service(UserServiceServer::new(UserService {
                bolt_pool: bb8::Pool::builder().build(bolt_manager).await?,
                auth_client: AuthServiceClient::connect(auth_url).await?,
            }))
            .serve_with_incoming(CombinedIncoming::new(
                (Ipv6Addr::UNSPECIFIED, 14514).into(),
                (Ipv4Addr::UNSPECIFIED, 14514).into(),
            )?)
            .await?;

        Ok(())
    }))
}

/// Build a runtime and block on a `Future`.
pub fn block_on<F: std::future::Future>(f: F) -> Result<F::Output, std::io::Error> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(f))
}
