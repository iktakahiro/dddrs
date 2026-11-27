---
name: implement-use-case
description: Use when adding or modifying domain behavior, a use case, persistence query, or HTTP endpoint on an existing entity in dddrs. Preserves ADT domain invariants, 1-struct-1-method use cases, SQLite Unit of Work concurrency rules, and FromRef handler wiring.
---

# Implement or Change a Use Case

Use this skill when adding or changing behavior on an existing entity (such as `todo`) across the five Onion Architecture crates. For creating a brand-new entity from scratch, use the `add-entity` skill instead.

## 1. Domain Layer (`crates/domain`)

- **Model invariants with types:** Keep entity fields private behind accessors. Represent mutually exclusive lifecycle data with an algebraic data type (e.g., `TodoState::Completed { completed_at }`) so illegal states cannot be constructed.
- **Value objects:** Wrap primitives in validated newtypes implementing `TryFrom` / `FromStr` (and normalization helpers such as `Description::from_optional` when emptiness has a single canonical form). Derive/implement common traits (`Debug, Clone, PartialEq, Eq, Hash, Display, AsRef<str>`).
- **Timestamps & determinism:** Use `chrono::DateTime<Utc>` in domain types (never raw Unix millisecond `i64`s). Provide `*_at(..., now: DateTime<Utc>)` helpers alongside `Utc::now()` convenience methods so unit tests are deterministic.
- **Identity & errors:** Implement entity `PartialEq` / `Eq` by identifier only and document that semantics. Return typed domain errors (`thiserror`) without HTTP or SQL concepts.
- **Repository ports:** Define methods on `&mut self` (for lock-free execution on an active transaction). Accept application limits (such as `limit: u32`) as parameters instead of hiding magic numbers in SQL. Reuse `domain::error::RepositoryError`.
- **Never** import `axum`, `sqlx`, or `serde`, and never return DB/JSON wire strings (e.g., `"not_started"`) from domain entities.

## 2. Application Layer (`crates/application`)

- **1 Struct = 1 Method:** Put each operation in its own file under `crates/application/src/<entity>/` with a `pub trait <Action><Entity>UseCase: Send + Sync` (`async fn execute(...)`) and a corresponding `pub struct <Action><Entity>UseCaseImpl`. Re-export both from `crates/application/src/<entity>.rs`.
- **Transaction boundaries (`UnitOfWork`):**
  - Use `self.uow_factory.begin().await?` for write use cases (acquires `BEGIN IMMEDIATE` on SQLite WAL).
  - Use `self.uow_factory.begin_read().await?` for read-only use cases (uses `BEGIN DEFERRED` so readers do not contend for the writer lock).
  - Always commit on success and explicitly roll back (`let _ = uow.rollback().await;`) on any error or after finishing a read-only snapshot.
  - Note for Rust Edition 2024 borrow scopes: bind `let res = uow.<repo>().op().await;` to a local variable before matching/branching so the temporary `Box<dyn ...Repository + '_>` borrow drops before calling `uow.rollback().await`.
- **Error isolation:** Translate `domain::error::RepositoryError` into `<entity>::UseCaseError` (`NotFound`, `Transition`, `Conflict`, `Unexpected`) so `presentation` never depends on `RepositoryError`.
- **Tests:** Add or update unit tests using the in-memory transactional `UnitOfWorkFactory` in `crates/application/src/<entity>/tests.rs` (declared via `#[cfg(test)] mod tests;` in `<entity>.rs` so large mock harnesses stay separate from production code while retaining access to private helpers). Keep compact unit tests inline in `#[cfg(test)] mod tests { ... }` when smaller than the production file.

## 3. Infrastructure Layer (`crates/infrastructure`)

- **Transaction-bound repository:** Implement the domain repository trait on `&mut Transaction<'static, Sqlite>` inside `crates/infrastructure/src/<entity>/repository.rs` and keep it `pub(crate)` behind `SqliteUnitOfWork`.
- **SQL hygiene:**
  - Always list columns explicitly via `SELECT_COLUMNS` (never `SELECT *`).
  - Parameterize limits (`LIMIT ?`) and preserve optimistic-locking checks (`WHERE id = ? AND version = ?`, returning `RepositoryError::Conflict` when `rows_affected() == 0`).
  - Map all `sqlx::Error` results with `crate::sqlite::sqlite_error` (not `storage`) so `SQLITE_BUSY` (`5`) and `SQLITE_BUSY_SNAPSHOT` (`517`) surface as `RepositoryError::Conflict` (`409`), never `500`.
  - Keep DB column string mappings (e.g., `status_column(state: TodoState)`) and `DateTime<Utc>` ↔ epoch-millisecond conversions inside `infrastructure`.
- **Row reconstitution:** Convert `TodoRow` via `TryFrom` calling `<Entity>::reconstitute(...)` in `row.rs`. If schema changes, add a new numbered SQL file under `crates/infrastructure/migrations/` (never edit an already-applied migration).
- **Tests:** Cover optimistic locking, rollback atomicity, and concurrent WAL behavior in `crates/infrastructure/tests/`.

## 4. Presentation Layer (`crates/presentation`)

- **Extractors:** Parse and validate path/body inputs into domain value objects in `extract.rs` (`FromRequestParts` / `FromRequest`), delegating normalization rules to domain constructors.
- **Single-use-case injection:** Add any new use case trait to `<Entity>UseCases` with a matching `FromRef<<Entity>UseCases>` impl in `handlers.rs`. Each handler must accept only `State<Arc<dyn <Single>UseCase>>`.
- **DTOs & error mapping:** Map domain states to response JSON strings and epoch milliseconds in `schema.rs`. Map `UseCaseError` and `ValidationError` to `ApiError` in `error.rs` without matching on `RepositoryError`.
- **Tests:** Add isolated handler unit tests using single-use-case stubs (mounting the single route with `.with_state(stub_usecase)`).

## 5. Server Wiring, Docs & Verification

- Wire new concrete `*UseCaseImpl` structs only in `crates/server/src/lib.rs` (`build_router`). Never duplicate DI wiring in `main.rs` or tests.
- Update `crates/server/tests/http.rs` (and `crates/server/tests/e2e/todos.hurl` if the public HTTP contract changes).
- Sync `README.md` and `README.ja_JP.md` 1:1 if API routes, status codes, field rules, or quoted code examples changed.
- Verify all gates pass:
  ```sh
  cargo fmt --all --check
  cargo clippy --workspace --all-targets --locked -- -D warnings
  cargo test --workspace --locked
  ```
