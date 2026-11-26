---
name: change-todo-use-case
description: Use when adding or changing Todo behavior or HTTP operations in dddrs. Keep domain rules inside their crate and test the affected boundaries.
---

# Change a Todo use case

Follow the [Rust API Guidelines for naming](https://rust-lang.github.io/api-guidelines/naming.html). Keep each layer's `lib.rs` limited to module declarations. Put Todo behavior under its `todo` module (for example, `domain::todo::Details` and `infrastructure::todo::SqliteTodoRepository`). Other entities get sibling modules, not branches in Todo code. Reuse the SQLite pool opened at startup.

1. Check the README's API table. Identify which behavior must match dddpy (lifecycle, status codes, character limits, list order) and which constraints can be expressed in Rust's types.
2. Put business rules in `crates/domain`. Validate newtypes on construction. Edit through `&mut self`; consume `self` for transitions that change the type. Add a `compile_fail` example for a newly prohibited call.
3. Keep `application` focused on loading, invoking the domain, and saving. Reuse existing repository methods when possible. For storage changes, update the row conversion and migration in `infrastructure` and preserve version checks for concurrent writes.
4. Translate input, output, and HTTP errors in `presentation`. Wire concrete repository implementations only in `server`.
5. Test the successful operation and verify that rejected operations do not persist changes. If the HTTP contract changes, update `crates/server/tests/http.rs` and the README examples. Write documentation, comments, and skills in English; keep `README.ja_JP.md` in sync as a translation.
6. Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo test --workspace --locked`. If the dependency test fails, reconsider where the behavior belongs before widening the allowlist.
