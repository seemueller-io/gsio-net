//! GSIO Client Library
//!
//! This library provides a client for interacting with GSIO nodes.
//! It allows connecting to nodes, adding entries to the ledger,
//! and retrieving ledger data.

use reqwest::{Client as HttpClient, Error as ReqwestError};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::time::Duration;
use thiserror::Error;
use tracing::{error, info};

/// Error type for GSIO client operations
#[derive(Error, Debug)]
pub enum GsioClientError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] ReqwestError),

    #[error("JSON serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Server error: {0}")]
    ServerError(String),
}

/// A ledger entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: String,
    pub timestamp: String,
    pub data: JsonValue,
    pub node_id: String,
    pub hash: String,
}

/// GSIO Client for interacting with GSIO nodes
pub struct GsioClient {
    client: HttpClient,
    node_url: String,
}

impl GsioClient {
    /// Create a new GSIO client
    pub fn new(node_url: &str) -> Result<Self, GsioClientError> {
        let client = HttpClient::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| GsioClientError::ConnectionError(e.to_string()))?;

        Ok(Self {
            client,
            node_url: node_url.to_string(),
        })
    }

    /// Add an entry to the ledger
    pub async fn add_ledger_entry(&self, data: JsonValue) -> Result<LedgerEntry, GsioClientError> {
        info!("Adding ledger entry: {:?}", data);

        let url = format!("{}/api/ledger", self.node_url);

        let response = self.client.post(&url)
            .json(&data)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(GsioClientError::ServerError(format!("Server returned error: {}", error_text)));
        }

        let entry: LedgerEntry = response.json().await?;

        Ok(entry)
    }

    /// Get all entries in the ledger
    pub async fn get_ledger(&self) -> Result<Vec<LedgerEntry>, GsioClientError> {
        info!("Getting ledger entries");

        let url = format!("{}/api/ledger", self.node_url);

        let response = self.client.get(&url)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(GsioClientError::ServerError(format!("Server returned error: {}", error_text)));
        }

        let entries: Vec<LedgerEntry> = response.json().await?;

        Ok(entries)
    }

    /// Get all known nodes in the network
    pub async fn get_known_nodes(&self) -> Result<Vec<String>, GsioClientError> {
        info!("Getting known nodes");

        let url = format!("{}/api/nodes", self.node_url);

        let response = self.client.get(&url)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(GsioClientError::ServerError(format!("Server returned error: {}", error_text)));
        }

        let data: JsonValue = response.json().await?;

        let nodes = data.get("nodes")
            .ok_or_else(|| GsioClientError::ServerError("Invalid response format".to_string()))?;

        let nodes: Vec<String> = serde_json::from_value(nodes.clone())?;

        Ok(nodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = GsioClient::new("http://localhost:3000").unwrap();
        assert_eq!(client.node_url, "http://localhost:3000");
    }

    // More tests would be added here in a real implementation
}
