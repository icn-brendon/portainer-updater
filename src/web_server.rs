use warp::Filter;
use log::{info, debug};
use std::env;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    let accept_route = warp::path("accept")
        .and(warp::get())
        .map(|| {
            info!("Accept endpoint hit");
            warp::reply::with_status("Accepted", warp::http::StatusCode::OK)
        });

    let reject_route = warp::path("reject")
        .and(warp::get())
        .map(|| {
            info!("Reject endpoint hit");
            warp::reply::with_status("Rejected", warp::http::StatusCode::OK)
        });

    let routes = accept_route.or(reject_route)
        .with(warp::log::custom(|info| {
            debug!("Received request: {} {}", info.method(), info.path());
            debug!("Response status: {}", info.status());
        }));

    let hostname = env::var("WEB_HOSTNAME").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = env::var("WEB_PORT").unwrap_or_else(|_| "3030".to_string()).parse().expect("PORT must be a number");

    let addr = format!("{}:{}", hostname, port);
    let socket_addr: SocketAddr = addr.parse().expect("Invalid hostname or port");

    warp::serve(routes)
        .run(socket_addr)
        .await;
}