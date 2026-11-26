use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};

/// Open a pool for all repositories and apply pending migrations.
pub async fn connect(url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = url.parse::<SqliteConnectOptions>()?.create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;
    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}
