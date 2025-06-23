use avail_rust::prelude::*;
use dotenvy::dotenv;
use std::env;
use serde_json::Value;
use reqwest::Client;

use avail::data_availability::storage::types::app_keys::Param0;

type ApplicationKeyCreatedEvent = avail::data_availability::events::ApplicationKeyCreated;
type DataSubmissionCall = avail::data_availability::calls::types::SubmitData;

const AVAIL_LIGHT_NODE_HTTP_URL: &str = "https://api.lightclient.turing.avail.so";
const AVAIL_LIGHT_NODE_WS_URL: &str = "wss://turing-rpc.avail.so/ws";

/// Load the AVAIL_SEED_PHRASE from .env and return an account
fn load_account_from_env() -> Result<Keypair, ClientError> {
    dotenv().ok();
    let seed = env::var("AVAIL_SEED_PHRASE")
        .map_err(|_| "Missing AVAIL_SEED_PHRASE environment variable")?;

    account::from_secret_uri(&seed)
}

/// Checks if an app ID (application key) exists on-chain by name
pub async fn does_app_id_exist_on_avail(app_name: &str) -> Result<Option<u32>, ClientError> {
    let sdk = SDK::new(AVAIL_LIGHT_NODE_WS_URL).await?;

    let key = Param0 { 0: app_name.as_bytes().to_vec() };

    let block_hash = sdk.client.best_block_hash().await?;
    let storage = sdk.client.storage().at(block_hash);

    let storage_key = avail::storage().data_availability().app_keys(key);
    let result = storage.fetch(&storage_key).await?;

    if let Some(app_key_info) = result {
        // app_key_info.id.0 is the app_id (u32)
        let app_id = app_key_info.id.0;
        Ok(Some(app_id))
    } else {
        Ok(None)
    }
}

/// Creates a new app ID on AvailDA
pub async fn create_app_id_on_avail(app_name: &str) -> Result<(), ClientError> {
    dotenv().ok();

    let account = load_account_from_env()?;

    let sdk = SDK::new(AVAIL_LIGHT_NODE_WS_URL).await?;

    let app_name_bytes = app_name.as_bytes().to_vec();

    let tx = sdk.tx.data_availability.create_application_key(app_name_bytes);
    let result = tx.execute_and_watch_inclusion(&account, Options::default()).await?;
    if result.is_successful() != Some(true) {
        return Err("Transaction failed".into());
    }

    let events = result.events.as_ref().unwrap();
    let event = events.find_first::<ApplicationKeyCreatedEvent>().unwrap();
    let Some(_event) = event else {
        return Err("Failed to find ApplicationKeyCreated event".into());
    };

    Ok(())
}

/// Submit a data to the AvailDA light node
pub async fn submit_data_to_avail_by_app_id(
    app_id: u32,
    data: String,
) -> Result<String, ClientError> {
    dotenv().ok();

    let account = load_account_from_env()?;

    let blob = String::from(data).into_bytes();

    let sdk = SDK::new(AVAIL_LIGHT_NODE_WS_URL).await?;
    let options = Options::new().app_id(app_id);

    let tx = sdk.tx.data_availability.submit_data(blob);
    let result = tx.execute_and_watch_inclusion(&account, options).await?;
    if result.is_successful() != Some(true) {
        return Err("Transaction failed".into());
    }

    let decoded = result.decode_as::<DataSubmissionCall>().await?;
    let Some(decoded) = decoded else {
        return Err("Failed to decode data submission call".into());
    };

    let data_decoded = to_ascii(decoded.data.0).unwrap();

    Ok(data_decoded)
}

/// Fetch the latest block height from the Avail light client HTTP API
pub async fn get_latest_block_height_on_avail() -> Result<u32, Box<dyn std::error::Error>> {
    let url = format!("{}/v2/status", AVAIL_LIGHT_NODE_HTTP_URL);
    let client = Client::new();

    let response = client
        .get(url)
        .header("User-Agent", "curl/7.88.1")
        .header("Accept", "application/json")
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await?;

    if status.is_success() {
        let parsed: Value = serde_json::from_str(&body)?;
        let latest_u64 = parsed["blocks"]["latest"]
            .as_u64()
            .ok_or("Missing or invalid `blocks.latest` in response")?;

        let latest_u32: u32 = latest_u64.try_into()
            .map_err(|_| "Block height too large to fit into u32")?;

        Ok(latest_u32)
    } else {
        Err(format!("Request failed with status: {}", status).into())
    }
}

/// Fetch the block hash for a given block height using the Avail WS client
pub async fn get_block_hash_by_height_on_avail(block_height: u32) -> Result<H256, ClientError> {
    let sdk = SDK::new(AVAIL_LIGHT_NODE_WS_URL).await?;

    let block_hash = rpc::chain::get_block_hash(&sdk.client, Some(block_height)).await?;

    Ok(block_hash)
}

/// Fetch and print blob data for a given app ID from a specific block hash
pub async fn get_block_data_by_hash_on_avail(
    block_hash: H256,
    app_id: u32
) -> Result<Vec<String>, ClientError> {
    let sdk = SDK::new(AVAIL_LIGHT_NODE_WS_URL).await?;
    let block = Block::new(&sdk.client, block_hash).await?;
    let blobs = block.data_submissions(Filter::new().app_id(app_id));

    let mut results = Vec::new();
    for blob in blobs.into_iter().rev() {
        let blob_data = blob.to_ascii().unwrap();
        results.push(blob_data);
    }
    Ok(results)
}

/// Fetch blob data for a given app ID from the latest N blocks and return all as a single string
pub async fn get_data_from_avail_by_app_id(
    app_id: u32,
    block_range: u32
) -> Result<Vec<String>, ClientError> {
    let latest_block_height = get_latest_block_height_on_avail()
        .await
        .map_err(|e| ClientError::from(e.to_string()))?;

    let mut all_data = vec![];

    let start_block_height = latest_block_height.saturating_sub(block_range);
    for block_height in (start_block_height..=latest_block_height).rev() {
        let block_hash = get_block_hash_by_height_on_avail(block_height).await?;
        let block_blobs = get_block_data_by_hash_on_avail(block_hash, app_id).await?;
        all_data.extend(block_blobs);
    }

    Ok(all_data)
}
