---
name: change-todo-use-case
description: Use when adding or changing Todo behavior or HTTP operations in dddrs. Keep domain rules inside their crate, preserve 1-struct-1-method use cases and Unit of Work boundaries, and test the affected layers.
---

# Change a Todo use case

Follow the [Rust API Guidelines for naming](https://rust-lang.github.io/api-guidelines/naming.html). Keep each layer's `lib.rs` limited to module declarations (shared cross-entity abstractions such as `domain::error`, `application::uow`, `infrastructure::sqlite`, and `presentation::error` live alongside entity modules). Put Todo behavior under its `todo` module. Other entities get sibling modules, not branches in Todo code.

1. Check the README's API table and behavior rules (lifecycle transitions, HTTP status codes, character limits, list order, optimistic locking).
2. Put business rules in `crates/domain`. Model entity state with algebraic data types (such as `TodoState`) so illegal states are unrepresentable, keep entity fields private behind accessors, use `DateTime<Utc>` for timestamps, and validate newtypes on construction (`TryFrom` / `FromStr` / `from_optional`).
3. Keep `crates/application` organized as **one struct and one `execute` method per use case** (with a corresponding `*UseCase` trait). Coordinate persistence through `UnitOfWorkFactory` / `UnitOfWork`: commit on success and roll back on error so multi-operation and multi-entity updates remain atomic. Add or update in-memory transactional unit tests in `crates/application`.
4. Implement persistence adapters in `crates/infrastructure` against the active SQLite transaction inside `SqliteUnitOfWork`. Use explicit SQL column lists, parameterized limits, and optimistic-locking version checks, and cover rollback/concurrency behavior in `crates/infrastructure/tests/`.
5. Translate HTTP inputs, outputs, and errors in `crates/presentation`. Inject a single use case trait per handler via axum `FromRef`. Wire concrete implementations only in `crates/server/src/lib.rs` (`build_router` / `build_sqlite_app`).
6. Write documentation, comments, and skills in English; keep `README.ja_JP.md` in sync as a natural Japanese translation.
7. Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo test --workspace --locked`. If `crates/server/tests/layers.rs` fails, reconsider where the code belongs before widening the dependency allowlist.
