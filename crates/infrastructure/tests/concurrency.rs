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

#[tokio::test]
async fn concurrent_immediate_writes_serialize_without_busy_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let url = format!(
        "sqlite://{}",
        dir.path().join("wal_concurrent.db").display()
    );
    let pool = sqlite::connect(&url).await.unwrap();
    let factory = SqliteUnitOfWorkFactory::new(pool);

    let todo = Todo::new(title("concurrent"), None);
    let id = todo.id();
    {
        let mut uow = factory.begin().await.unwrap();
        uow.todos().insert(&todo).await.unwrap();
        uow.commit().await.unwrap();
    }

    // Hold an open BEGIN IMMEDIATE transaction while a second writer starts concurrently.
    let mut first_uow = factory.begin().await.unwrap();
    let mut first_snapshot = first_uow.todos().find_by_id(id).await.unwrap().unwrap();
    first_snapshot.todo.start().unwrap();
    first_uow
        .todos()
        .update(&first_snapshot.todo, first_snapshot.version)
        .await
        .unwrap();

    let second_factory = factory.clone();
    let second_writer = tokio::spawn(async move {
        let mut uow = second_factory.begin().await.unwrap();
        let mut snapshot = uow.todos().find_by_id(id).await.unwrap().unwrap();
        assert_eq!(snapshot.todo.state(), TodoState::InProgress);
        snapshot.todo.complete().unwrap();
        uow.todos()
            .update(&snapshot.todo, snapshot.version)
            .await
            .unwrap();
        uow.commit().await.unwrap();
    });

    // Give the second task a chance to hit BEGIN IMMEDIATE and wait on busy_timeout.
    tokio::task::yield_now().await;
    first_uow.commit().await.unwrap();
    second_writer.await.unwrap();

    let mut verify_uow = factory.begin_read().await.unwrap();
    let final_snapshot = verify_uow.todos().find_by_id(id).await.unwrap().unwrap();
    assert!(matches!(
        final_snapshot.todo.state(),
        TodoState::Completed { .. }
    ));
    assert_eq!(final_snapshot.version, 2);
    verify_uow.rollback().await.unwrap();
}

#[tokio::test]
async fn deferred_snapshot_upgrade_maps_sqlite_busy_snapshot_to_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("wal_snapshot.db").display());
    let pool = sqlite::connect(&url).await.unwrap();
    let factory = SqliteUnitOfWorkFactory::new(pool);

    let todo = Todo::new(title("snapshot"), None);
    let id = todo.id();
    {
        let mut uow = factory.begin().await.unwrap();
        uow.todos().insert(&todo).await.unwrap();
        uow.commit().await.unwrap();
    }

    // Acquire a WAL read snapshot via BEGIN DEFERRED.
    let mut deferred_uow = factory.begin_read().await.unwrap();
    let _ = deferred_uow.todos().find_by_id(id).await.unwrap().unwrap();

    // Commit a concurrent write from another connection while the read snapshot is held.
    {
        let mut writer_uow = factory.begin().await.unwrap();
        let mut current = writer_uow.todos().find_by_id(id).await.unwrap().unwrap();
        current.todo.start().unwrap();
        writer_uow
            .todos()
            .update(&current.todo, current.version)
            .await
            .unwrap();
        writer_uow.commit().await.unwrap();
    }

    // Upgrading the stale deferred snapshot triggers SQLITE_BUSY_SNAPSHOT (517),
    // which must map to RepositoryError::Conflict rather than Storage (500).
    // Use insert() here because it has no rows_affected() == 0 Conflict path,
    // ensuring Conflict comes strictly from SQLITE_BUSY_SNAPSHOT.
    let another = Todo::new(title("inserted on stale snapshot"), None);
    let result = deferred_uow.todos().insert(&another).await;
    assert!(matches!(result, Err(RepositoryError::Conflict)));
    deferred_uow.rollback().await.unwrap();
}
