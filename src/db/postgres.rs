use anyhow::{Context, Result};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;

#[derive(Debug)]
pub struct PostgresDb {
    pub pool: PgPool,
}

impl PostgresDb {
    /// Creates a new connection pool for PostgreSQL.
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(3))
            .connect(database_url)
            .await
            .with_context(|| format!("Failed to connect to PostgreSQL at {}", database_url))?;

        Ok(Self { pool })
    }

    /// Runs all pending migrations.
    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .context("Failed to run database migrations")?;
        Ok(())
    }
}
