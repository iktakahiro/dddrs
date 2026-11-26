use domain::todo::{RepositoryError, StoredTodo, Title, Todo, TodoRepository};
use infrastructure::{sqlite, todo::SqliteTodoRepository};

#[tokio::test]
async fn a_stale_write_cannot_undo_a_transition_or_resurrect_a_deleted_todo() {
    let pool = sqlite::connect("sqlite::memory:").await.unwrap();
    let repository = SqliteTodoRepository::new(pool.clone());
    let other = SqliteTodoRepository::new(pool);
    let todo = StoredTodo::NotStarted(Todo::new(Title::try_from("read".to_owned()).unwrap(), None));
    let id = todo.details().id;
    repository.insert(&todo).await.unwrap();
    let first = repository.find(id).await.unwrap().unwrap();
    let stale = other.find(id).await.unwrap().unwrap();
    repository
        .update(&first.todo.start().unwrap(), first.version)
        .await
        .unwrap();
    assert!(matches!(
        other.update(&stale.todo, stale.version).await,
        Err(RepositoryError::Conflict)
    ));
    let latest = other.find(id).await.unwrap().unwrap();
    assert_eq!(latest.todo.status(), "in_progress");
    assert!(repository.delete(id).await.unwrap());
    assert!(matches!(
        other.update(&latest.todo, latest.version).await,
        Err(RepositoryError::Conflict)
    ));
    assert!(repository.find(id).await.unwrap().is_none());
}
