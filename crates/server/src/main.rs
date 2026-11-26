use std::{env, error::Error};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://todos.db".into());
    let addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let app = server::build_sqlite_app(&url).await?;
    let listener = TcpListener::bind(addr).await?;
    eprintln!("Listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
