use std::{
    env,
    error::Error,
    future::Future,
    net::{Ipv4Addr, Ipv6Addr},
    pin::Pin,
    time::Duration,
};

use combind_incoming::CombinedIncoming;
use health_check::HealthChecker;
use proto::health_checker_server::HealthCheckerServer;
use tokio::task::JoinHandle;
use tonic::{transport::Server, Response, Status};

mod combind_incoming;
mod health_check;
pub mod proto;

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

    let mut postgres_config = tokio_postgres::config::Config::new();
    postgres_config
        .options(&env::var("POSTGRES_URL").map_err(|_| "POSTGRES_URL doesn't exist.".to_string())?);

    Ok(tokio::spawn(async move {
        Server::builder()
            .concurrency_limit_per_connection(256)
            .tcp_keepalive(Some(Duration::from_secs(10)))
            .add_service(HealthCheckerServer::new(HealthChecker))
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
