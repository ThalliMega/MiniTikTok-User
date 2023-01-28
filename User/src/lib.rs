use std::{
    collections::HashMap,
    env,
    error::Error,
    future::Future,
    net::{Ipv4Addr, Ipv6Addr},
    pin::Pin,
    time::Duration,
};

use combind_incoming::CombinedIncoming;
use health_check::HealthChecker;
use proto::{health_checker_server::HealthCheckerServer, user_service_server::UserServiceServer};
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
pub fn start_up() -> Result<JoinHandle<Result<(), DynError>>, String> {
    env_logger::init();

    let bolt_metadata: bolt_client::Metadata = HashMap::from([
        ("user_agent", "MiniTikTok-User/0"),
        ("scheme", "basic"),
        (
            "principal",
            // TODO: String::leak
            Box::leak(
                env::var("BOLT_USERNAME")
                    .map_err(|_| "BOLT_USERNAME doesn't exist.".to_string())?
                    .into_boxed_str(),
            ),
        ),
        (
            "credentials",
            // TODO: String::leak
            Box::leak(
                env::var("BOLT_PASSWORD")
                    .map_err(|_| "BOLT_PASSWORD doesn't exist.".to_string())?
                    .into_boxed_str(),
            ),
        ),
    ])
    .into();

    let bolt_url = env::var("BOLT_URL").map_err(|_| "BOLT_URL doesn't exist.".to_string())?;

    let bolt_domain = env::var("BOLT_DOMAIN").ok();

    Ok(tokio::spawn(async move {
        Server::builder()
            .concurrency_limit_per_connection(256)
            .tcp_keepalive(Some(Duration::from_secs(10)))
            .add_service(HealthCheckerServer::new(HealthChecker))
            .add_service(UserServiceServer::new(UserService {
                bolt_domain,
                bolt_metadata,
                bolt_url,
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
