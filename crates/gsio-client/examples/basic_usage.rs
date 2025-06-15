use gsio_client::{GsioClient, GsioClientError};
use serde_json::json;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), GsioClientError> {
    // Initialize tracing
    tracing::subscriber::set_global_default(FmtSubscriber::default())
        .expect("Failed to set tracing subscriber");

    // Create a new client
    let client = GsioClient::new("http://localhost:3000")?;
    println!("Created GSIO client");

    // Add an entry to the ledger
    let entry_data = json!({
        "message": "Hello, GSIO!",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    let entry = client.add_ledger_entry(entry_data).await?;
    println!("Added ledger entry: {:?}", entry);

    // Get all entries in the ledger
    let entries = client.get_ledger().await?;
    println!("Ledger entries:");
    for entry in entries {
        println!("  - {}: {}", entry.id, entry.data);
    }

    // Get all known nodes
    let nodes = client.get_known_nodes().await?;
    println!("Known nodes:");
    for node in nodes {
        println!("  - {}", node);
    }

    println!("Example completed successfully");

    Ok(())
}
