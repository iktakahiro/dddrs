use application::uow::UnitOfWorkFactory;
use domain::{
    error::RepositoryError,
    todo::{Title, Todo, TodoState},
};
use infrastructure::sqlite::{self, SqliteUnitOfWorkFactory};

fn title(raw: &str) -> Title {
    Title::try_from(raw.to_owned()).unwrap()
}

#[tokio::test]
async fn a_stale_write_cannot_undo_a_transition_or_resurrect_a_deleted_todo() {
    let pool = sqlite::connect("sqlite::memory:").await.unwrap();
    let factory = SqliteUnitOfWorkFactory::new(pool);

    let todo = Todo::new(title("read"), None);
    let id = todo.id();

    {
        let mut uow = factory.begin().await.unwrap();
        uow.todos().insert(&todo).await.unwrap();
        uow.commit().await.unwrap();
    }

    let first = {
        let mut uow = factory.begin().await.unwrap();
        let snapshot = uow.todos().find_by_id(id).await.unwrap().unwrap();
        uow.rollback().await.unwrap();
        snapshot
    };
    let stale = {
        let mut uow = factory.begin().await.unwrap();
        let snapshot = uow.todos().find_by_id(id).await.unwrap().unwrap();
        uow.rollback().await.unwrap();
        snapshot
    };

    // First writer transitions and commits (version 0 -> 1).
    {
        let mut uow = factory.begin().await.unwrap();
        let mut started = first.todo;
        started.start().unwrap();
        uow.todos().update(&started, first.version).await.unwrap();
        uow.commit().await.unwrap();
    }

    // Stale writer still holds version 0 and must be rejected with Conflict.
    {
        let mut uow = factory.begin().await.unwrap();
        let result = uow.todos().update(&stale.todo, stale.version).await;
        assert!(matches!(result, Err(RepositoryError::Conflict)));
        uow.rollback().await.unwrap();
    }

    let latest = {
        let mut uow = factory.begin().await.unwrap();
        let snapshot = uow.todos().find_by_id(id).await.unwrap().unwrap();
        uow.rollback().await.unwrap();
        snapshot
    };
    assert_eq!(latest.todo.state(), TodoState::InProgress);

    // Delete the Todo and ensure a stale update cannot resurrect it.
    {
        let mut uow = factory.begin().await.unwrap();
        assert!(uow.todos().delete(id).await.unwrap());
        uow.commit().await.unwrap();
    }
    {
        let mut uow = factory.begin().await.unwrap();
        let result = uow.todos().update(&latest.todo, latest.version).await;
        assert!(matches!(result, Err(RepositoryError::Conflict)));
        uow.rollback().await.unwrap();
    }
    {
        let mut uow = factory.begin().await.unwrap();
        assert!(uow.todos().find_by_id(id).await.unwrap().is_none());
        uow.rollback().await.unwrap();
    }
}

#[tokio::test]
async fn rolled_back_and_uncommitted_transactions_discard_all_writes() {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("uow.db").display());
    let pool = sqlite::connect(&url).await.unwrap();
    let factory = SqliteUnitOfWorkFactory::new(pool);

    // Explicit rollback discards multiple inserts atomically.
    let first = Todo::new(title("first"), None);
    let second = Todo::new(title("second"), None);
    {
        let mut uow = factory.begin().await.unwrap();
        uow.todos().insert(&first).await.unwrap();
        uow.todos().insert(&second).await.unwrap();
        uow.rollback().await.unwrap();
    }
    {
        let mut uow = factory.begin().await.unwrap();
        assert!(uow.todos().find_latest(20).await.unwrap().is_empty());
        uow.rollback().await.unwrap();
    }

    // Commit persists both inserts atomically.
    {
        let mut uow = factory.begin().await.unwrap();
        uow.todos().insert(&first).await.unwrap();
        uow.todos().insert(&second).await.unwrap();
        uow.commit().await.unwrap();
    }

    // Dropping a UnitOfWork without commit rolls back the pending update.
    {
        let mut uow = factory.begin().await.unwrap();
        let mut snapshot = uow.todos().find_by_id(first.id()).await.unwrap().unwrap();
        snapshot.todo.start().unwrap();
        uow.todos()
            .update(&snapshot.todo, snapshot.version)
            .await
            .unwrap();
        // Intentionally drop without committing.
    }

    {
        let mut uow = factory.begin().await.unwrap();
        let reloaded = uow.todos().find_by_id(first.id()).await.unwrap().unwrap();
        assert_eq!(reloaded.todo.state(), TodoState::NotStarted);
        assert_eq!(reloaded.version, 0);
        assert_eq!(uow.todos().find_latest(20).await.unwrap().len(), 2);
        uow.rollback().await.unwrap();
    }
}
