use tokio_postgres::{NoTls, Error};
use dotenv::dotenv;
use std::env;
use anyhow::{Result, anyhow};
use log::{info, error};
use env_logger;
use reqwest::Client;

mod version_fetcher;

async fn connect_to_db() -> Result<tokio_postgres::Client, Error> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    info!("Connecting to database at {}", database_url);
    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });
    Ok(client)
}

async fn fetch_containers(client: &tokio_postgres::Client) -> Result<Vec<(String, String, String, String, String)>, Error> {
    info!("Fetching containers from database");
    let rows = client.query("SELECT webhook_url, version, namespace, repository, image_source FROM containers", &[]).await?;
    let mut data = Vec::new();
    for row in rows {
        let webhook_url: String = row.get(0);
        let version: String = row.get(1);
        let namespace: String = row.get(2);
        let repository: String = row.get(3);
        let image_source: String = row.get(4);
        data.push((webhook_url, version, namespace, repository, image_source));
    }
    info!("Fetched containers data: {:?}", data);
    Ok(data)
}

async fn update_version_in_db(client: &tokio_postgres::Client, webhook_url: &str, new_version: &str) -> Result<(), Error> {
    info!("Updating version in database for {} to {}", webhook_url, new_version);
    client.execute(
        "UPDATE containers SET version = $1 WHERE webhook_url = $2",
        &[&new_version, &webhook_url],
    ).await?;
    Ok(())
}

async fn send_upgrade_notification(webhook_url: &str) -> Result<(), anyhow::Error> {
    info!("Sending upgrade notification to {}", webhook_url);

    // Create a reqwest client that accepts self-signed certificates
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| anyhow!("Failed to build HTTP client: {}", e))?;

    let res = client.post(webhook_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body("Redeploy with latest image of same tag")
        .send()
        .await
        .map_err(|e| anyhow!("Failed to send POST request: {}", e))?;

    if res.status().is_success() {
        info!("Webhook request sent successfully to: {}", webhook_url);
        println!("Webhook request sent to: {}", webhook_url);
    } else {
        error!("Failed to send webhook request: {}", res.status());
        return Err(anyhow!("Failed to send webhook request: {}", res.status()));
    }

    Ok(())
}

fn is_semver(version: &str) -> bool {
    version.split('.').all(|part| part.parse::<u64>().is_ok())
}

fn compare_versions(current_version: &str, latest_version: &str) -> bool {
    if is_semver(current_version) && is_semver(latest_version) {
        let current_parts: Vec<u64> = current_version.split('.').map(|part| part.parse().unwrap()).collect();
        let latest_parts: Vec<u64> = latest_version.split('.').map(|part| part.parse().unwrap()).collect();
        current_parts < latest_parts
    } else {
        current_version != latest_version
    }
}

async fn check_and_update() -> Result<(), Box<dyn std::error::Error>> {
    let db_client = connect_to_db().await?;
    let data = fetch_containers(&db_client).await?;

    for (webhook_url, version, namespace, repository, image_source) in data {
        let latest_version = version_fetcher::fetch_latest_version(&namespace, &repository, &image_source, &version).await?;
        info!("Fetched latest version for {}/{}: {}", namespace, repository, latest_version);

        // Log the versions being compared
        info!("Comparing current version: {} with latest version: {}", version, latest_version);

        if compare_versions(&version, &latest_version) {
            info!("Updating {} from version {} to {}", webhook_url, version, latest_version);
            if let Err(e) = send_upgrade_notification(&webhook_url).await {
                error!("Failed to send upgrade notification: {}", e);
                continue; // Skip updating the database if the notification fails
            }
            update_version_in_db(&db_client, &webhook_url, &latest_version).await?;
            println!("Triggered upgrade for {}", webhook_url);
        } else {
            info!("No update needed for {} as the version {} is up-to-date", webhook_url, version);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    env_logger::init();

    check_and_update().await?;

    Ok(())
}