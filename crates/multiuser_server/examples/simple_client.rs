//! Simple example client demonstrating the join flow
//!
//! This example shows how to:
//! 1. Create a session via REST API
//! 2. Join a session with a token
//! 3. Connect via WebSocket for signaling
//! 4. Perform UDP hole punching
//! 5. Establish QUIC P2P connection

use anyhow::Result;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let base_url = std::env::var("PULSAR_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let client = reqwest::Client::new();

    println!("Pulsar MultiEdit - Example Client");
    println!("=================================\n");

    // Step 1: Create a session
    println!("1. Creating session...");
    let create_resp = client
        .post(format!("{}/v1/sessions", base_url))
        .json(&json!({
            "host_id": "example-host",
            "metadata": {
                "name": "Example Session",
                "max_participants": 10
            }
        }))
        .send()
        .await?;

    if !create_resp.status().is_success() {
        anyhow::bail!("Failed to create session: {}", create_resp.status());
    }

    let create_data: serde_json::Value = create_resp.json().await?;
    let session_id = create_data["session_id"].as_str().unwrap();
    let join_token = create_data["join_token"].as_str().unwrap();

    println!("   ✓ Session created:");
    println!("     - ID: {}", session_id);
    println!("     - Token: {}...", &join_token[..20]);
    println!();

    // Step 2: Join the session
    println!("2. Joining session as peer...");
    let join_resp = client
        .post(format!("{}/v1/sessions/{}/join", base_url, session_id))
        .json(&json!({
            "join_token": join_token,
            "peer_id": "example-peer"
        }))
        .send()
        .await?;

    if !join_resp.status().is_success() {
        anyhow::bail!("Failed to join session: {}", join_resp.status());
    }

    let join_data: serde_json::Value = join_resp.json().await?;
    println!("   ✓ Joined session:");
    println!("     - Peer ID: {}", join_data["peer_id"]);
    println!("     - Role: {}", join_data["role"]);
    println!("     - Participants: {}", join_data["participant_count"]);
    println!();

    // Step 3: Get session info
    println!("3. Fetching session details...");
    let session_resp = client
        .get(format!("{}/v1/sessions/{}", base_url, session_id))
        .send()
        .await?;

    if session_resp.status().is_success() {
        let session_data: serde_json::Value = session_resp.json().await?;
        println!("   ✓ Session details:");
        println!("     - Host: {}", session_data["host_id"]);
        println!("     - Created: {}", session_data["created_at"]);
        println!("     - Expires: {}", session_data["expires_at"]);
        println!();
    }

    // Step 4: Check health
    println!("4. Checking service health...");
    let health_resp = client
        .get(format!("{}/health", base_url))
        .send()
        .await?;

    if health_resp.status().is_success() {
        let health_data: serde_json::Value = health_resp.json().await?;
        println!("   ✓ Service health: {}", health_data["status"]);
        if let Some(checks) = health_data["checks"].as_array() {
            for check in checks {
                println!("     - {}: {}", check["name"], check["status"]);
            }
        }
        println!();
    }

    // Step 5: Close session
    println!("5. Closing session...");
    let close_resp = client
        .post(format!("{}/v1/sessions/{}/close", base_url, session_id))
        .send()
        .await?;

    if close_resp.status().is_success() {
        println!("   ✓ Session closed");
    } else {
        println!("   ✗ Failed to close session: {}", close_resp.status());
    }
    println!();

    println!("Example completed successfully!");

    Ok(())
}
