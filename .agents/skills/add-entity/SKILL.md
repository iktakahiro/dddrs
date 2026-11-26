---
name: add-entity
description: Use when introducing a new domain entity or aggregate (for example, Project, Tag, or User) into dddrs or a project derived from it. Scaffolds sibling modules across all five crates, extends UnitOfWork for cross-aggregate atomicity, adds migrations, and merges the new axum router.
---

# Add a New Entity / Aggregate

Use this skill when adding a brand-new sibling entity (e.g., `project`) to the workspace. Do not add conditional branches inside `todo` modules; place the new entity in sibling modules across every layer and reuse the shared cross-entity modules.

## Reuse Shared Modules (Do Not Duplicate)

Before creating files, note which abstractions are already entity-agnostic:
- `domain::error::RepositoryError`
- `application::uow::{UnitOfWork, UnitOfWorkFactory}`
- `infrastructure::sqlite::{connect, sqlite_error, storage, SqliteUnitOfWork, SqliteUnitOfWorkFactory}`
- `presentation::error::ApiError`

## Step-by-Step Blueprint (Example: `project`)

### 1. Domain (`crates/domain`)
- Declare `pub mod project;` in `crates/domain/src/lib.rs`.
- Create:
  - `crates/domain/src/project.rs`: module declarations and scoped `pub use` re-exports (`domain::project::*`) so short names like `Title` or `Snapshot` never collide with `domain::todo::*`.
  - `crates/domain/src/project/entity.rs`: encapsulated `Project` aggregate root, ADT state if stateful, `reconstitute(...)`, identity-based `PartialEq`/`Eq`, and deterministic `*_at` unit tests.
  - `crates/domain/src/project/values.rs`: `ProjectId` (`Uuid`), validated newtype value objects, `ValidationError`, and unit tests.
  - `crates/domain/src/project/repository.rs`: `Snapshot { pub project: Project, pub version: i64 }` (if using optimistic locking) and `#[async_trait] pub trait ProjectRepository: Send` taking `&mut self` and returning `domain::error::RepositoryError`.

### 2. Application (`crates/application`)
- Extend `UnitOfWork` in `crates/application/src/uow.rs` with a repository accessor so cross-entity use cases can commit or roll back multiple aggregates atomically:
  ```rust
  fn projects(&mut self) -> Box<dyn ProjectRepository + '_>;
  ```
- Declare `pub mod project;` in `crates/application/src/lib.rs`.
- Create `crates/application/src/project.rs` and `crates/application/src/project/`:
  - `error.rs`: `UseCaseError` translating `RepositoryError` into `Conflict` / `Unexpected`.
  - One file per operation (`create_project.rs`, `find_project_by_id.rs`, ...) with one `*UseCase` trait and one `*UseCaseImpl` struct (`execute` method only).
  - Use `begin()` for writes and `begin_read()` for reads.
  - Add in-memory transactional `UnitOfWorkFactory` unit tests (including rollback tests if a use case spans multiple entities, e.g., `project` + `todo`).

### 3. Infrastructure (`crates/infrastructure`)
- Add a new numbered migration `crates/infrastructure/migrations/000N_projects.sql`:
  - Include `version INTEGER NOT NULL DEFAULT 0` if using optimistic locking.
  - Foreign keys are enforced (`foreign_keys(true)` in `sqlite::connect`), so declare `REFERENCES` constraints where appropriate and index lookup/sort columns.
- Declare `mod project;` in `crates/infrastructure/src/lib.rs` (kept private behind `SqliteUnitOfWork`).
- Create:
  - `crates/infrastructure/src/project.rs` re-exporting `pub(crate) use repository::SqliteProjectRepository;`.
  - `crates/infrastructure/src/project/row.rs` (`#[derive(sqlx::FromRow)]` + `TryFrom<ProjectRow> for Snapshot`).
  - `crates/infrastructure/src/project/repository.rs` implementing `ProjectRepository` on `&mut Transaction<'static, Sqlite>`, using explicit `SELECT_COLUMNS`, parameterized `LIMIT ?`, and `crate::sqlite::sqlite_error`.
- Wire `fn projects(&mut self)` in `SqliteUnitOfWork` (`crates/infrastructure/src/sqlite.rs`).
- Add SQLite integration tests in `crates/infrastructure/tests/` covering rollback, optimistic locking, and WAL concurrency.

### 4. Presentation (`crates/presentation`)
- Declare `pub mod project;` in `crates/presentation/src/lib.rs`.
- Create `crates/presentation/src/project.rs` and `crates/presentation/src/project/`:
  - `extract.rs`: custom `FromRequestParts` / `FromRequest` extractors producing domain value objects.
  - `schema.rs`: `ProjectResponse` (`Serialize`) + `From<Project>`.
  - `error.rs`: `From<ValidationError>` and `From<UseCaseError>` for `crate::error::ApiError`.
  - `handlers.rs`: `ProjectUseCases`, `FromRef<ProjectUseCases>` per use case trait, `pub fn router(usecases: ProjectUseCases) -> Router`, and single-stub handler unit tests.

### 5. Server Wiring & Verification (`crates/server`)
- In `crates/server/src/lib.rs` (`build_router`), instantiate `ProjectUseCases` sharing the existing `Arc<dyn UnitOfWorkFactory>` and chain `.merge(presentation::project::router(project_usecases))`.
- Add HTTP integration tests in `crates/server/tests/http.rs` (including database reopen and concurrent WAL writes if applicable).
- Confirm `crates/server/tests/layers.rs` passes without weakening any dependency rules.
- Update `README.md` and `README.ja_JP.md` 1:1 if adding public endpoints or workspace files.
- Run:
  ```sh
  cargo fmt --all --check
  cargo clippy --workspace --all-targets --locked -- -D warnings
  cargo test --workspace --locked
  ```
