use application::TodoService;
use infrastructure::SqliteTodoRepository;
use std::{env, error::Error, sync::Arc};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://todos.db".into());
    let addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let repository = SqliteTodoRepository::connect(&url).await?;
    let app = presentation::router(TodoService::new(Arc::new(repository)));
    let listener = TcpListener::bind(addr).await?;
    eprintln!("Listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
