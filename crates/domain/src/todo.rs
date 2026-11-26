//! Todo aggregate, value objects, and repository port.

mod entity;
mod repository;
mod values;

pub use entity::{Todo, TodoState, TransitionError};
pub use repository::{Snapshot, TodoRepository};
pub use values::{Description, Title, TodoId, ValidationError};
