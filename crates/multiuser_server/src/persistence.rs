//! Database and storage persistence layer
//!
//! This module handles database operations with PostgreSQL via sqlx
//! and S3 integration for snapshot storage.

use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::{primitives::ByteStream, Client as S3Client};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::config::Config;
use crate::metrics::METRICS;

/// Session record in database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SessionRecord {
    pub id: Uuid,
    pub session_id: String,
    pub host_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub status: String,
    pub metadata: serde_json::Value,
}

/// Snapshot metadata
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SnapshotRecord {
    pub id: Uuid,
    pub snapshot_id: String,
    pub session_id: String,
    pub s3_key: String,
    pub s3_bucket: String,
    pub size_bytes: i64,
    pub compressed: bool,
    pub hash: Vec<u8>,
    pub created_at: DateTime<Utc>,
}

/// Persistence layer
pub struct PersistenceLayer {
    db_pool: Option<Arc<PgPool>>,
    s3_client: Option<Arc<S3Client>>,
    s3_bucket: Option<String>,
    config: Config,
}

impl PersistenceLayer {
    /// Initialize persistence layer with database and S3
    pub async fn new(config: Config) -> Result<Self> {
        let db_pool = if let Some(ref db_url) = config.database_url {
            info!("Connecting to PostgreSQL database");
            let pool = Self::create_pool(db_url).await?;
            Self::run_migrations(&pool).await?;
            METRICS.database_health.set(1.0);
            Some(Arc::new(pool))
        } else {
            warn!("No database URL configured - persistence disabled");
            METRICS.database_health.set(0.0);
            None
        };

        let (s3_client, s3_bucket) = if let Some(ref bucket) = config.s3_bucket {
            info!(bucket = %bucket, "Initializing S3 client");
            let client = Self::create_s3_client(&config).await?;
            METRICS.s3_health.set(1.0);
            (Some(Arc::new(client)), Some(bucket.clone()))
        } else {
            warn!("No S3 bucket configured - snapshot storage disabled");
            METRICS.s3_health.set(0.0);
            (None, None)
        };

        Ok(Self {
            db_pool,
            s3_client,
            s3_bucket,
            config,
        })
    }

    async fn create_pool(database_url: &str) -> Result<PgPool> {
        PgPoolOptions::new()
            .max_connections(20)
            .min_connections(2)
            .acquire_timeout(Duration::from_secs(5))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(1800))
            .connect(database_url)
            .await
            .context("Failed to create database pool")
    }

    async fn run_migrations(pool: &PgPool) -> Result<()> {
        info!("Running database migrations");

        // Create sessions table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                session_id VARCHAR(255) UNIQUE NOT NULL,
                host_id VARCHAR(255) NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                expires_at TIMESTAMPTZ NOT NULL,
                closed_at TIMESTAMPTZ,
                status VARCHAR(50) NOT NULL DEFAULT 'active',
                metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
                CONSTRAINT valid_status CHECK (status IN ('active', 'closed', 'expired'))
            );
            "#,
        )
        .execute(pool)
        .await
        .context("Failed to create sessions table")?;

        // Create index on session_id
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_sessions_session_id ON sessions(session_id);
            "#,
        )
        .execute(pool)
        .await
        .context("Failed to create session_id index")?;

        // Create index on status for active sessions
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status) WHERE status = 'active';
            "#,
        )
        .execute(pool)
        .await
        .context("Failed to create status index")?;

        // Create snapshots table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS snapshots (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                snapshot_id VARCHAR(255) UNIQUE NOT NULL,
                session_id VARCHAR(255) NOT NULL,
                s3_key VARCHAR(1024) NOT NULL,
                s3_bucket VARCHAR(255) NOT NULL,
                size_bytes BIGINT NOT NULL,
                compressed BOOLEAN NOT NULL DEFAULT false,
                hash BYTEA NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
            );
            "#,
        )
        .execute(pool)
        .await
        .context("Failed to create snapshots table")?;

        // Create index on session_id for snapshots
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_snapshots_session_id ON snapshots(session_id);
            "#,
        )
        .execute(pool)
        .await
        .context("Failed to create snapshots session_id index")?;

        info!("Database migrations completed");
        Ok(())
    }

    async fn create_s3_client(config: &Config) -> Result<S3Client> {
        let aws_config = if let Some(ref region) = config.s3_region {
            aws_config::defaults(BehaviorVersion::latest())
                .region(aws_sdk_s3::config::Region::new(region.clone()))
                .load()
                .await
        } else {
            aws_config::load_defaults(BehaviorVersion::latest()).await
        };

        Ok(S3Client::new(&aws_config))
    }

    /// Create a new session record
    pub async fn create_session(
        &self,
        session_id: String,
        host_id: String,
        expires_at: DateTime<Utc>,
        metadata: serde_json::Value,
    ) -> Result<SessionRecord> {
        let pool = self
            .db_pool
            .as_ref()
            .context("Database not configured")?;

        let record = sqlx::query_as::<_, SessionRecord>(
            r#"
            INSERT INTO sessions (session_id, host_id, expires_at, metadata)
            VALUES ($1, $2, $3, $4)
            RETURNING id, session_id, host_id, created_at, expires_at, closed_at, status, metadata
            "#,
        )
        .bind(&session_id)
        .bind(&host_id)
        .bind(expires_at)
        .bind(metadata)
        .fetch_one(pool.as_ref())
        .await
        .context("Failed to create session")?;

        info!(session_id = %session_id, "Created session record");

        Ok(record)
    }

    /// Get a session by ID
    pub async fn get_session(&self, session_id: &str) -> Result<Option<SessionRecord>> {
        let pool = self
            .db_pool
            .as_ref()
            .context("Database not configured")?;

        let record = sqlx::query_as::<_, SessionRecord>(
            r#"
            SELECT id, session_id, host_id, created_at, expires_at, closed_at, status, metadata
            FROM sessions
            WHERE session_id = $1
            "#,
        )
        .bind(session_id)
        .fetch_optional(pool.as_ref())
        .await
        .context("Failed to get session")?;

        Ok(record)
    }

    /// Update session status
    pub async fn update_session_status(&self, session_id: &str, status: &str) -> Result<()> {
        let pool = self
            .db_pool
            .as_ref()
            .context("Database not configured")?;

        sqlx::query(
            r#"
            UPDATE sessions
            SET status = $1, closed_at = CASE WHEN $1 = 'closed' THEN NOW() ELSE closed_at END
            WHERE session_id = $2
            "#,
        )
        .bind(status)
        .bind(session_id)
        .execute(pool.as_ref())
        .await
        .context("Failed to update session status")?;

        debug!(session_id = %session_id, status = %status, "Updated session status");

        Ok(())
    }

    /// Close a session
    pub async fn close_session(&self, session_id: &str) -> Result<()> {
        self.update_session_status(session_id, "closed").await
    }

    /// List active sessions
    pub async fn list_active_sessions(&self) -> Result<Vec<SessionRecord>> {
        let pool = self
            .db_pool
            .as_ref()
            .context("Database not configured")?;

        let records = sqlx::query_as::<_, SessionRecord>(
            r#"
            SELECT id, session_id, host_id, created_at, expires_at, closed_at, status, metadata
            FROM sessions
            WHERE status = 'active' AND expires_at > NOW()
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool.as_ref())
        .await
        .context("Failed to list active sessions")?;

        Ok(records)
    }

    /// Expire old sessions
    pub async fn expire_old_sessions(&self) -> Result<u64> {
        let pool = self
            .db_pool
            .as_ref()
            .context("Database not configured")?;

        let result = sqlx::query(
            r#"
            UPDATE sessions
            SET status = 'expired', closed_at = NOW()
            WHERE status = 'active' AND expires_at <= NOW()
            "#,
        )
        .execute(pool.as_ref())
        .await
        .context("Failed to expire old sessions")?;

        let count = result.rows_affected();
        if count > 0 {
            info!(count = count, "Expired old sessions");
        }

        Ok(count)
    }

    /// Upload snapshot to S3
    pub async fn upload_snapshot(
        &self,
        snapshot_id: String,
        session_id: String,
        data: Vec<u8>,
        compressed: bool,
    ) -> Result<SnapshotRecord> {
        let s3_client = self.s3_client.as_ref().context("S3 not configured")?;
        let s3_bucket = self.s3_bucket.as_ref().context("S3 bucket not set")?;

        // Generate S3 key
        let s3_key = format!("snapshots/{}/{}.bin", session_id, snapshot_id);

        // Calculate hash
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(&data).to_vec();

        // Upload to S3
        info!(
            bucket = %s3_bucket,
            key = %s3_key,
            size = data.len(),
            "Uploading snapshot to S3"
        );

        let body = ByteStream::from(data.clone());
        s3_client
            .put_object()
            .bucket(s3_bucket)
            .key(&s3_key)
            .body(body)
            .send()
            .await
            .context("Failed to upload snapshot to S3")?;

        // Store metadata in database
        let pool = self
            .db_pool
            .as_ref()
            .context("Database not configured")?;

        let record = sqlx::query_as::<_, SnapshotRecord>(
            r#"
            INSERT INTO snapshots (snapshot_id, session_id, s3_key, s3_bucket, size_bytes, compressed, hash)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, snapshot_id, session_id, s3_key, s3_bucket, size_bytes, compressed, hash, created_at
            "#,
        )
        .bind(&snapshot_id)
        .bind(&session_id)
        .bind(&s3_key)
        .bind(s3_bucket)
        .bind(data.len() as i64)
        .bind(compressed)
        .bind(&hash)
        .fetch_one(pool.as_ref())
        .await
        .context("Failed to create snapshot record")?;

        info!(snapshot_id = %snapshot_id, "Created snapshot record");

        Ok(record)
    }

    /// Download snapshot from S3
    pub async fn download_snapshot(&self, snapshot_id: &str) -> Result<Vec<u8>> {
        let s3_client = self.s3_client.as_ref().context("S3 not configured")?;
        let pool = self
            .db_pool
            .as_ref()
            .context("Database not configured")?;

        // Get snapshot metadata
        let record = sqlx::query_as::<_, SnapshotRecord>(
            r#"
            SELECT id, snapshot_id, session_id, s3_key, s3_bucket, size_bytes, compressed, hash, created_at
            FROM snapshots
            WHERE snapshot_id = $1
            "#,
        )
        .bind(snapshot_id)
        .fetch_one(pool.as_ref())
        .await
        .context("Snapshot not found")?;

        // Download from S3
        info!(
            bucket = %record.s3_bucket,
            key = %record.s3_key,
            "Downloading snapshot from S3"
        );

        let response = s3_client
            .get_object()
            .bucket(&record.s3_bucket)
            .key(&record.s3_key)
            .send()
            .await
            .context("Failed to download snapshot from S3")?;

        let data = response
            .body
            .collect()
            .await
            .context("Failed to read snapshot data")?
            .into_bytes()
            .to_vec();

        // Verify hash
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(&data).to_vec();
        if hash != record.hash {
            anyhow::bail!("Snapshot hash mismatch");
        }

        Ok(data)
    }

    /// Delete snapshot
    pub async fn delete_snapshot(&self, snapshot_id: &str) -> Result<()> {
        let s3_client = self.s3_client.as_ref().context("S3 not configured")?;
        let pool = self
            .db_pool
            .as_ref()
            .context("Database not configured")?;

        // Get snapshot metadata
        let record = sqlx::query_as::<_, SnapshotRecord>(
            r#"
            SELECT id, snapshot_id, session_id, s3_key, s3_bucket, size_bytes, compressed, hash, created_at
            FROM snapshots
            WHERE snapshot_id = $1
            "#,
        )
        .bind(snapshot_id)
        .fetch_one(pool.as_ref())
        .await
        .context("Snapshot not found")?;

        // Delete from S3
        s3_client
            .delete_object()
            .bucket(&record.s3_bucket)
            .key(&record.s3_key)
            .send()
            .await
            .context("Failed to delete snapshot from S3")?;

        // Delete from database
        sqlx::query(
            r#"
            DELETE FROM snapshots WHERE snapshot_id = $1
            "#,
        )
        .bind(snapshot_id)
        .execute(pool.as_ref())
        .await
        .context("Failed to delete snapshot record")?;

        info!(snapshot_id = %snapshot_id, "Deleted snapshot");

        Ok(())
    }

    /// Health check for database
    pub async fn check_database_health(&self) -> Result<bool> {
        if let Some(ref pool) = self.db_pool {
            match sqlx::query("SELECT 1").execute(pool.as_ref()).await {
                Ok(_) => {
                    METRICS.database_health.set(1.0);
                    Ok(true)
                }
                Err(e) => {
                    error!(error = %e, "Database health check failed");
                    METRICS.database_health.set(0.0);
                    Ok(false)
                }
            }
        } else {
            Ok(false)
        }
    }

    /// Health check for S3
    pub async fn check_s3_health(&self) -> Result<bool> {
        if let (Some(ref client), Some(ref bucket)) = (&self.s3_client, &self.s3_bucket) {
            match client.head_bucket().bucket(bucket).send().await {
                Ok(_) => {
                    METRICS.s3_health.set(1.0);
                    Ok(true)
                }
                Err(e) => {
                    error!(error = %e, "S3 health check failed");
                    METRICS.s3_health.set(0.0);
                    Ok(false)
                }
            }
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_persistence_layer_creation() {
        let config = Config::default();
        let result = PersistenceLayer::new(config).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_session_record_serialization() {
        let record = SessionRecord {
            id: Uuid::new_v4(),
            session_id: "test-session".to_string(),
            host_id: "host-1".to_string(),
            created_at: Utc::now(),
            expires_at: Utc::now(),
            closed_at: None,
            status: "active".to_string(),
            metadata: serde_json::json!({}),
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("test-session"));
    }
}
