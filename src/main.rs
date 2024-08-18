use tokio_postgres::{NoTls, Error};
use reqwest::ClientBuilder;
use dotenv::dotenv;
use std::env;
use anyhow::Result;
use log::info;
use semver::Version;

mod version_fetcher;

async fn connect_to_db() -> Result<tokio_postgres::Client, Error> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });
    Ok(client)
}

async fn fetch_webhook_urls_and_versions(client: &tokio_postgres::Client) -> Result<Vec<(String, String)>, Error> {
    let rows = client.query("SELECT webhook_url, version FROM containers", &[]).await?;
    let mut data = Vec::new();
    for row in rows {
        let webhook_url: String = row.get(0);
        let version: String = row.get(1);
        data.push((webhook_url, version));
    }
    Ok(data)
}

async fn fetch_repositories(client: &tokio_postgres::Client) -> Result<Vec<(String, String, String)>, Error> {
    let rows = client.query("SELECT namespace, repository, image_source FROM repositories", &[]).await?;
    let mut data = Vec::new();
    for row in rows {
        let namespace: String = row.get(0);
        let repository: String = row.get(1);
        let image_source: String = row.get(2);
        data.push((namespace, repository, image_source));
    }
    Ok(data)
}

async fn update_version_in_db(client: &tokio_postgres::Client, webhook_url: &str, new_version: &str) -> Result<(), Error> {
    client.execute(
        "UPDATE containers SET version = $1 WHERE webhook_url = $2",
        &[&new_version, &webhook_url],
    ).await?;
    Ok(())
}

async fn check_and_update() -> Result<(), Box<dyn std::error::Error>> {
    let db_client = connect_to_db().await?;
    let data = fetch_webhook_urls_and_versions(&db_client).await?;
    let repositories = fetch_repositories(&db_client).await?;

    info!("Fetched webhook URLs and versions: {:?}", data);
    info!("Fetched repositories: {:?}", repositories);

    let ghcr_token = env::var("GHCR_TOKEN").ok();

    for (namespace, repository, image_source) in repositories {
        let latest_version = version_fetcher::fetch_latest_version(&namespace, &repository, &image_source, ghcr_token.as_deref()).await?;
        info!("Fetched latest version for {}/{}: {}", namespace, repository, latest_version);
        for (webhook_url, version) in &data {
            if Version::parse(version)? < Version::parse(&latest_version)? {
                info!("Updating {} from version {} to {}", webhook_url, version, latest_version);
                update_version_in_db(&db_client, &webhook_url, &latest_version).await?;
                println!("Triggered upgrade for {}", webhook_url);
            } else {
                info!("No update needed for {} as the version {} is up-to-date", webhook_url, version);
            }
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