use domain::todo::{RepositoryError, StoredTodo, Title, Todo, TodoRepository};
use infrastructure::SqliteTodoRepository;

#[tokio::test]
async fn a_stale_write_cannot_undo_a_transition_or_resurrect_a_deleted_todo() {
    let repository = SqliteTodoRepository::connect("sqlite::memory:")
        .await
        .unwrap();
    let todo = StoredTodo::NotStarted(Todo::new(Title::try_from("read".to_owned()).unwrap(), None));
    let id = todo.details().id;
    repository.insert(&todo).await.unwrap();
    let first = repository.find(id).await.unwrap().unwrap();
    let stale = repository.find(id).await.unwrap().unwrap();
    repository
        .update(&first.todo.start().unwrap(), first.version)
        .await
        .unwrap();
    assert!(matches!(
        repository.update(&stale.todo, stale.version).await,
        Err(RepositoryError::Conflict)
    ));
    let latest = repository.find(id).await.unwrap().unwrap();
    assert_eq!(latest.todo.status(), "in_progress");
    assert!(repository.delete(id).await.unwrap());
    assert!(matches!(
        repository.update(&latest.todo, latest.version).await,
        Err(RepositoryError::Conflict)
    ));
    assert!(repository.find(id).await.unwrap().is_none());
}
