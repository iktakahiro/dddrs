# Repository Guidelines (`dddrs`)

`dddrs` is a reference Rust workspace demonstrating **Domain-Driven Design (DDD)** and **Onion Architecture** with `axum`, `sqlx` (SQLite), and Edition 2024. It is designed as a base template that scales cleanly to multiple entities and complex models.

## 1. Architecture & Layer Boundaries

Each Onion Architecture layer lives in its own Cargo crate. Allowed workspace dependencies are enforced in CI by `crates/server/tests/layers.rs`:

| Crate | Allowed workspace dependencies | Primary role |
| --- | --- | --- |
| `crates/domain` | None | Entities, ADT state models, validated value objects, domain errors, repository traits |
| `crates/application` | `domain` | 1-struct-1-method use cases (`*UseCase` + `*UseCaseImpl`), `UnitOfWork` / `UnitOfWorkFactory` ports, `UseCaseError` |
| `crates/infrastructure` | `application`, `domain` | SQLite pool setup, `SqliteUnitOfWorkFactory` / `SqliteUnitOfWork`, transaction-bound repositories, row reconstitution, migrations |
| `crates/presentation` | `application`, `domain` | `axum` routers, custom extractors, `FromRef` single-use-case handlers, response DTOs, HTTP error mapping |
| `crates/server` | `presentation`, `application`, `infrastructure` | Composition root (`src/lib.rs`), binary entrypoint (`src/main.rs`), integration, layer, and E2E tests |

Hard guardrails:
- Never import `axum`, `sqlx`, or `serde` into `domain` or `application`.
- Never import `sqlx` outside `infrastructure`, or `axum` outside `presentation` and `server`.
- Keep each layer's `lib.rs` limited to module declarations (`pub mod ...` / `mod ...`); group entity code in sibling modules (`todo`, `project`, ...) alongside shared cross-entity modules (`domain::error`, `application::uow`, `infrastructure::sqlite`, `presentation::error`).
- Do not widen the dependency allowlist in `crates/server/tests/layers.rs` to work around a placement mistake—reconsider which layer owns the behavior instead.

## 2. Communication, Documentation & Naming

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/naming.html) for naming and common traits.
- Write all code comments, docstrings, commit messages, pull request titles/bodies, and agent skills in **English**.
- Keep `README.md` (English) and `README.ja_JP.md` (natural polite Japanese) synchronized 1:1 in structure, links, and code examples whenever architecture, behavior, or tooling changes.

## 3. Dependency Hygiene

- `.cargo/config.toml` enforces a 7-day publish-age cooldown (`registry.global-min-publish-age = "7 days"`, Cargo 1.100+ / RFC 3923).
- Always run Cargo checks with `--locked` unless intentionally updating dependencies.
- For an urgent security patch inside the cooldown window, override narrowly:
  `CARGO_RESOLVER_INCOMPATIBLE_PUBLISH_AGE=allow cargo update -p <crate> --precise <version>`

## 4. Quality Gates

Run before every commit and pull request:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

When changing HTTP routes, status codes, or response schemas, also verify the live-server Hurl E2E test (when `hurl` is installed):

```sh
cargo test --locked -p server --test e2e -- --ignored --nocapture
```

## 5. Task-Specific Agent Skills

Use the focused skills under `.agents/skills/` for recurring workflows:

- `.agents/skills/implement-use-case/SKILL.md`: Adding or changing domain behavior, use cases, persistence, or HTTP operations on an existing entity.
- `.agents/skills/add-entity/SKILL.md`: Introducing a new sibling entity/aggregate across all five crates, `UnitOfWork`, migrations, and router composition.
- `.agents/skills/conventional-commits/SKILL.md`: Structuring Conventional Commits v1.0.0 messages, splitting commits cleanly, and preparing English PRs.
