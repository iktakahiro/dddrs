use super::values::{Description, Title, TodoId};
use chrono::{DateTime, Utc};
use thiserror::Error;

/// Lifecycle state of a Todo.
///
/// Storing `completed_at` inside the `Completed` variant makes illegal states
/// unrepresentable: an incomplete Todo cannot carry a completion timestamp, and
/// a completed Todo always carries one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoState {
    NotStarted,
    InProgress,
    Completed { completed_at: DateTime<Utc> },
}

impl TodoState {
    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        match self {
            Self::Completed { completed_at } => Some(*completed_at),
            Self::NotStarted | Self::InProgress => None,
        }
    }
}

/// Domain error returned when an invalid lifecycle transition is requested.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TransitionError {
    #[error("Todo has not been started")]
    NotStarted,
    #[error("Todo has already been started")]
    AlreadyStarted,
    #[error("Todo has already been completed")]
    AlreadyCompleted,
}

/// Todo aggregate root.
///
/// `PartialEq` and `Eq` compare only [`TodoId`], following DDD entity identity
/// semantics rather than structural field equality.
#[derive(Debug, Clone)]
pub struct Todo {
    id: TodoId,
    title: Title,
    description: Option<Description>,
    state: TodoState,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Todo {
    /// Create a new Todo in the `NotStarted` state.
    pub fn new(title: Title, description: Option<Description>) -> Self {
        Self::new_at(title, description, Utc::now())
    }

    /// Create a new Todo with an explicit timestamp (useful in deterministic tests).
    pub fn new_at(title: Title, description: Option<Description>, now: DateTime<Utc>) -> Self {
        Self {
            id: TodoId::generate(),
            title,
            description,
            state: TodoState::NotStarted,
            created_at: now,
            updated_at: now,
        }
    }

    /// Reconstitute a Todo from persisted state.
    pub fn reconstitute(
        id: TodoId,
        title: Title,
        description: Option<Description>,
        state: TodoState,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            title,
            description,
            state,
            created_at,
            updated_at,
        }
    }

    pub fn id(&self) -> TodoId {
        self.id
    }

    pub fn title(&self) -> &Title {
        &self.title
    }

    pub fn description(&self) -> Option<&Description> {
        self.description.as_ref()
    }

    pub fn state(&self) -> TodoState {
        self.state
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.state.completed_at()
    }

    /// Replace the Todo's title and description.
    pub fn update(&mut self, title: Title, description: Option<Description>) {
        self.update_at(title, description, Utc::now());
    }

    /// Replace the Todo's title and description at an explicit timestamp.
    pub fn update_at(
        &mut self,
        title: Title,
        description: Option<Description>,
        now: DateTime<Utc>,
    ) {
        self.title = title;
        self.description = description;
        self.updated_at = now;
    }

    /// Transition the Todo from `NotStarted` to `InProgress`.
    pub fn start(&mut self) -> Result<(), TransitionError> {
        self.start_at(Utc::now())
    }

    /// Transition the Todo from `NotStarted` to `InProgress` at an explicit timestamp.
    pub fn start_at(&mut self, now: DateTime<Utc>) -> Result<(), TransitionError> {
        match self.state {
            TodoState::NotStarted => {
                self.state = TodoState::InProgress;
                self.updated_at = now;
                Ok(())
            }
            TodoState::InProgress => Err(TransitionError::AlreadyStarted),
            TodoState::Completed { .. } => Err(TransitionError::AlreadyCompleted),
        }
    }

    /// Transition the Todo from `InProgress` to `Completed`.
    pub fn complete(&mut self) -> Result<(), TransitionError> {
        self.complete_at(Utc::now())
    }

    /// Transition the Todo from `InProgress` to `Completed` at an explicit timestamp.
    pub fn complete_at(&mut self, now: DateTime<Utc>) -> Result<(), TransitionError> {
        match self.state {
            TodoState::NotStarted => Err(TransitionError::NotStarted),
            TodoState::InProgress => {
                self.state = TodoState::Completed { completed_at: now };
                self.updated_at = now;
                Ok(())
            }
            TodoState::Completed { .. } => Err(TransitionError::AlreadyCompleted),
        }
    }
}

impl PartialEq for Todo {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Todo {}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(millis: i64) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(millis).unwrap()
    }

    fn title(raw: &str) -> Title {
        Title::try_from(raw.to_owned()).unwrap()
    }

    #[test]
    fn valid_lifecycle_transitions_update_state_and_timestamps() {
        let mut todo = Todo::new_at(title("Buy groceries"), None, ts(1_000));
        assert_eq!(todo.state(), TodoState::NotStarted);
        assert_eq!(todo.created_at(), ts(1_000));
        assert_eq!(todo.updated_at(), ts(1_000));
        assert_eq!(todo.completed_at(), None);

        todo.start_at(ts(2_000)).unwrap();
        assert_eq!(todo.state(), TodoState::InProgress);
        assert_eq!(todo.updated_at(), ts(2_000));
        assert_eq!(todo.completed_at(), None);

        todo.complete_at(ts(3_000)).unwrap();
        assert_eq!(
            todo.state(),
            TodoState::Completed {
                completed_at: ts(3_000),
            }
        );
        assert_eq!(todo.updated_at(), ts(3_000));
        assert_eq!(todo.completed_at(), Some(ts(3_000)));
        assert_eq!(todo.created_at(), ts(1_000));
    }

    #[test]
    fn invalid_lifecycle_transitions_are_rejected_without_mutating_state() {
        let mut todo = Todo::new_at(title("Read book"), None, ts(1_000));

        assert_eq!(
            todo.complete_at(ts(2_000)),
            Err(TransitionError::NotStarted)
        );
        assert_eq!(todo.state(), TodoState::NotStarted);
        assert_eq!(todo.updated_at(), ts(1_000));

        todo.start_at(ts(2_000)).unwrap();
        assert_eq!(
            todo.start_at(ts(3_000)),
            Err(TransitionError::AlreadyStarted)
        );
        assert_eq!(todo.updated_at(), ts(2_000));

        todo.complete_at(ts(4_000)).unwrap();
        assert_eq!(
            todo.start_at(ts(5_000)),
            Err(TransitionError::AlreadyCompleted)
        );
        assert_eq!(
            todo.complete_at(ts(5_000)),
            Err(TransitionError::AlreadyCompleted)
        );
        assert_eq!(todo.completed_at(), Some(ts(4_000)));
    }

    #[test]
    fn update_replaces_title_and_description_and_refreshes_updated_at() {
        let mut todo = Todo::new_at(title("Initial"), None, ts(1_000));
        let desc = Description::try_from("Details".to_owned()).unwrap();

        todo.update_at(title("Updated"), Some(desc.clone()), ts(2_000));
        assert_eq!(todo.title().as_str(), "Updated");
        assert_eq!(todo.description(), Some(&desc));
        assert_eq!(todo.updated_at(), ts(2_000));

        todo.update_at(title("Cleared"), None, ts(3_000));
        assert_eq!(todo.description(), None);
        assert_eq!(todo.updated_at(), ts(3_000));
    }

    #[test]
    fn entity_equality_uses_identifier_only() {
        let first = Todo::new_at(title("First"), None, ts(1_000));
        let renamed = Todo::reconstitute(
            first.id(),
            title("Renamed"),
            None,
            TodoState::InProgress,
            ts(1_000),
            ts(2_000),
        );
        let other = Todo::new_at(title("First"), None, ts(1_000));

        assert_eq!(first, renamed);
        assert_ne!(first, other);
    }
}
