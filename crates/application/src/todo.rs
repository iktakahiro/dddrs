//! Todo application use cases, one struct and one `execute` method per use case.

mod complete_todo;
mod create_todo;
mod delete_todo;
mod error;
mod find_todo_by_id;
mod find_todos;
mod start_todo;
mod update_todo;

pub use complete_todo::{CompleteTodoUseCase, CompleteTodoUseCaseImpl};
pub use create_todo::{CreateTodoUseCase, CreateTodoUseCaseImpl};
pub use delete_todo::{DeleteTodoUseCase, DeleteTodoUseCaseImpl};
pub use error::UseCaseError;
pub use find_todo_by_id::{FindTodoByIdUseCase, FindTodoByIdUseCaseImpl};
pub use find_todos::{DEFAULT_LIST_LIMIT, FindTodosUseCase, FindTodosUseCaseImpl};
pub use start_todo::{StartTodoUseCase, StartTodoUseCaseImpl};
pub use update_todo::{UpdateTodoUseCase, UpdateTodoUseCaseImpl};

use crate::uow::UnitOfWorkFactory;
use domain::todo::{Todo, TodoId, TransitionError};

/// Load a Todo, apply a domain mutation, persist with optimistic locking, and commit.
async fn update_in_tx(
    uow_factory: &dyn UnitOfWorkFactory,
    id: TodoId,
    mutate: impl FnOnce(&mut Todo) -> Result<(), TransitionError>,
) -> Result<Todo, UseCaseError> {
    let mut uow = uow_factory.begin().await?;
    let result: Result<Todo, UseCaseError> = async {
        let snapshot = uow
            .todos()
            .find_by_id(id)
            .await?
            .ok_or(UseCaseError::NotFound)?;
        let mut todo = snapshot.todo;
        mutate(&mut todo)?;
        uow.todos().update(&todo, snapshot.version).await?;
        Ok(todo)
    }
    .await;

    match result {
        Ok(todo) => {
            uow.commit().await?;
            Ok(todo)
        }
        Err(error) => {
            let _ = uow.rollback().await;
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uow::{UnitOfWork, UnitOfWorkFactory};
    use async_trait::async_trait;
    use domain::{
        error::RepositoryError,
        todo::{Description, Snapshot, Title, Todo, TodoId, TodoRepository, TodoState},
    };
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    #[derive(Clone, Default)]
    struct StoreState {
        records: HashMap<TodoId, (Todo, i64, u64)>,
        next_seq: u64,
        fail_next_update_with_conflict: bool,
        commits: usize,
        rollbacks: usize,
    }

    #[derive(Clone, Default)]
    struct InMemoryUowFactory {
        state: Arc<Mutex<StoreState>>,
    }

    impl InMemoryUowFactory {
        fn stats(&self) -> (usize, usize) {
            let state = self.state.lock().unwrap();
            (state.commits, state.rollbacks)
        }

        fn fail_next_update_with_conflict(&self) {
            self.state.lock().unwrap().fail_next_update_with_conflict = true;
        }
    }

    struct InMemoryUow {
        shared: Arc<Mutex<StoreState>>,
        draft_records: HashMap<TodoId, (Todo, i64, u64)>,
        draft_next_seq: u64,
        fail_update_with_conflict: bool,
    }

    struct InMemoryTodoRepository<'a> {
        records: &'a mut HashMap<TodoId, (Todo, i64, u64)>,
        next_seq: &'a mut u64,
        fail_update_with_conflict: &'a mut bool,
    }

    #[async_trait]
    impl UnitOfWorkFactory for InMemoryUowFactory {
        async fn begin(&self) -> Result<Box<dyn UnitOfWork>, RepositoryError> {
            let mut state = self.state.lock().unwrap();
            let fail = state.fail_next_update_with_conflict;
            state.fail_next_update_with_conflict = false;
            Ok(Box::new(InMemoryUow {
                shared: Arc::clone(&self.state),
                draft_records: state.records.clone(),
                draft_next_seq: state.next_seq,
                fail_update_with_conflict: fail,
            }))
        }
    }

    #[async_trait]
    impl UnitOfWork for InMemoryUow {
        fn todos(&mut self) -> Box<dyn TodoRepository + '_> {
            Box::new(InMemoryTodoRepository {
                records: &mut self.draft_records,
                next_seq: &mut self.draft_next_seq,
                fail_update_with_conflict: &mut self.fail_update_with_conflict,
            })
        }

        async fn commit(self: Box<Self>) -> Result<(), RepositoryError> {
            let mut state = self.shared.lock().unwrap();
            state.records = self.draft_records;
            state.next_seq = self.draft_next_seq;
            state.commits += 1;
            Ok(())
        }

        async fn rollback(self: Box<Self>) -> Result<(), RepositoryError> {
            let mut state = self.shared.lock().unwrap();
            state.rollbacks += 1;
            Ok(())
        }
    }

    #[async_trait]
    impl TodoRepository for InMemoryTodoRepository<'_> {
        async fn insert(&mut self, todo: &Todo) -> Result<(), RepositoryError> {
            let seq = *self.next_seq;
            *self.next_seq += 1;
            self.records.insert(todo.id(), (todo.clone(), 0, seq));
            Ok(())
        }

        async fn find_by_id(&mut self, id: TodoId) -> Result<Option<Snapshot>, RepositoryError> {
            Ok(self.records.get(&id).map(|(todo, version, _)| Snapshot {
                todo: todo.clone(),
                version: *version,
            }))
        }

        async fn find_latest(&mut self, limit: u32) -> Result<Vec<Todo>, RepositoryError> {
            let mut items: Vec<_> = self.records.values().collect();
            items.sort_by(|a, b| {
                b.0.created_at()
                    .cmp(&a.0.created_at())
                    .then_with(|| b.2.cmp(&a.2))
            });
            Ok(items
                .into_iter()
                .take(limit as usize)
                .map(|(todo, _, _)| todo.clone())
                .collect())
        }

        async fn update(
            &mut self,
            todo: &Todo,
            expected_version: i64,
        ) -> Result<(), RepositoryError> {
            if *self.fail_update_with_conflict {
                return Err(RepositoryError::Conflict);
            }
            match self.records.get_mut(&todo.id()) {
                Some((stored, version, _)) if *version == expected_version => {
                    *stored = todo.clone();
                    *version += 1;
                    Ok(())
                }
                _ => Err(RepositoryError::Conflict),
            }
        }

        async fn delete(&mut self, id: TodoId) -> Result<bool, RepositoryError> {
            Ok(self.records.remove(&id).is_some())
        }
    }

    fn title(raw: &str) -> Title {
        Title::try_from(raw.to_owned()).unwrap()
    }

    #[tokio::test]
    async fn full_todo_use_case_lifecycle_and_rollback_semantics() {
        let factory = Arc::new(InMemoryUowFactory::default());
        let uow: Arc<dyn UnitOfWorkFactory> = factory.clone();

        let create = CreateTodoUseCaseImpl::new(Arc::clone(&uow));
        let find_by_id = FindTodoByIdUseCaseImpl::new(Arc::clone(&uow));
        let find_todos = FindTodosUseCaseImpl::new(Arc::clone(&uow));
        let update = UpdateTodoUseCaseImpl::new(Arc::clone(&uow));
        let start = StartTodoUseCaseImpl::new(Arc::clone(&uow));
        let complete = CompleteTodoUseCaseImpl::new(Arc::clone(&uow));
        let delete = DeleteTodoUseCaseImpl::new(Arc::clone(&uow));

        let created = create.execute(title("Write docs"), None).await.unwrap();
        assert_eq!(created.state(), TodoState::NotStarted);

        // Premature completion rolls back without mutating stored state.
        let err = complete.execute(created.id()).await.unwrap_err();
        assert!(matches!(
            err,
            UseCaseError::Transition(TransitionError::NotStarted)
        ));
        let loaded = find_by_id.execute(created.id()).await.unwrap();
        assert_eq!(loaded.state(), TodoState::NotStarted);

        // Update, start, and complete commit cleanly.
        let desc = Description::try_from("Include diagrams".to_owned()).unwrap();
        let edited = update
            .execute(created.id(), title("Write README"), Some(desc.clone()))
            .await
            .unwrap();
        assert_eq!(edited.title().as_str(), "Write README");
        assert_eq!(edited.description(), Some(&desc));

        let started = start.execute(created.id()).await.unwrap();
        assert_eq!(started.state(), TodoState::InProgress);

        // Optimistic lock conflict rolls back and surfaces UseCaseError::Conflict.
        factory.fail_next_update_with_conflict();
        let conflict = complete.execute(created.id()).await.unwrap_err();
        assert!(matches!(conflict, UseCaseError::Conflict));
        assert_eq!(
            find_by_id.execute(created.id()).await.unwrap().state(),
            TodoState::InProgress
        );

        let completed = complete.execute(created.id()).await.unwrap();
        assert!(matches!(completed.state(), TodoState::Completed { .. }));
        assert!(completed.completed_at().is_some());

        let listed = find_todos.execute().await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id(), created.id());

        delete.execute(created.id()).await.unwrap();
        assert!(matches!(
            find_by_id.execute(created.id()).await.unwrap_err(),
            UseCaseError::NotFound
        ));
        assert!(matches!(
            delete.execute(created.id()).await.unwrap_err(),
            UseCaseError::NotFound
        ));

        let (commits, rollbacks) = factory.stats();
        assert_eq!(commits, 5); // create, update, start, complete, delete
        assert!(rollbacks >= 4); // failed complete, conflict, missing delete, reads
    }

    #[tokio::test]
    async fn find_todos_applies_default_limit() {
        let factory = Arc::new(InMemoryUowFactory::default());
        let uow: Arc<dyn UnitOfWorkFactory> = factory;
        let create = CreateTodoUseCaseImpl::new(Arc::clone(&uow));
        let find_todos = FindTodosUseCaseImpl::new(uow);

        for i in 0..(DEFAULT_LIST_LIMIT + 3) {
            create
                .execute(title(&format!("Task {i}")), None)
                .await
                .unwrap();
        }

        let listed = find_todos.execute().await.unwrap();
        assert_eq!(listed.len(), DEFAULT_LIST_LIMIT as usize);
    }
}
