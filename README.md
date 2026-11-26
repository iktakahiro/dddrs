# dddrs

English | [日本語](README.ja_JP.md)

A Rust, axum, and SQLite implementation of the [dddpy](https://github.com/iktakahiro/dddpy) Todo API. The example follows one domain rule: **a Todo must be started before it can be completed**. When its state is known, Rust enforces the rule at compile time. When the state comes from a database row, the domain checks it at runtime.

## Run the API

Install [stable Rust](https://rustup.rs/) and a C compiler. You don't need a separate SQLite server or the sqlx CLI.

```sh
cargo run --locked -p server
```

The server creates `todos.db` in the working directory and runs migrations on startup. Set `DATABASE_URL` to change the database (default: `sqlite://todos.db`) or `BIND_ADDR` to change the listener (default: `127.0.0.1:3000`).

In a second terminal, create a Todo and try completing it before and after starting it. The example uses `jq` to read the ID.

```sh
base=http://127.0.0.1:3000/todos
id=$(curl -fsS "$base" -H 'Content-Type: application/json' \
  -d '{"title":"Buy groceries","description":"Pick up vegetables"}' | jq -r .id)
curl -sS -i -X PATCH "$base/$id/complete"  # 400: not started yet
curl -fsS -X PATCH "$base/$id/start"       # in_progress
curl -fsS -X PATCH "$base/$id/complete"    # completed
curl -fsS "$base/$id"
```

## Follow a request

Trace `PATCH /todos/{id}/start` to see why **runtime calls and compile-time dependencies point in different directions**:

1. The [presentation handler](crates/presentation/src/lib.rs) receives the request. Its [extractor](crates/presentation/src/extract.rs) parses the path into a `TodoId`.
2. In [application](crates/application/src/lib.rs), `TodoService::start` reads the Todo through the [repository trait](crates/domain/src/todo/repository.rs) and asks the [domain](crates/domain/src/todo.rs) to start it.
3. The [infrastructure adapter](crates/infrastructure/src/lib.rs) implements that trait with sqlx. The update only succeeds if the stored version still matches the one read, so a stale write cannot undo a later transition.
4. Presentation converts the result to JSON. Only the [server](crates/server/src/main.rs) chooses the concrete repository, once at startup.

| Crate | Direct workspace dependencies |
| --- | --- |
| domain | None |
| application | domain |
| infrastructure | domain |
| presentation | application, domain |
| server (startup and wiring) | presentation, application, infrastructure |

Each crate declares its dependencies in its own `Cargo.toml`. Importing `infrastructure::SqliteTodoRepository` from application fails to compile because infrastructure isn't a declared dependency. The [dependency test](crates/server/tests/layers.rs) also catches an attempt to relax the boundary by adding it to the manifest.

The domain groups public types by entity rather than re-exporting them at the crate root. For example, `domain::todo::Details` and `domain::todo::Title` belong to a Todo; another entity can use the same short names in its own module.

## What Rust enforces

```rust
use domain::todo::{Title, Todo};

let title = Title::try_from(String::from("Buy groceries")).unwrap();
let pending = Todo::new(title, None);   // Todo<NotStarted>
// pending.complete();                 // No such method on this type.
let active = pending.start();          // Moves pending; returns Todo<InProgress>.
// pending.start();                    // The previous value is no longer available.
let done = active.complete();          // Todo<Completed> carries the completion time.
```

| In dddpy | In dddrs |
| --- | --- |
| Value objects are frozen dataclasses. | [Title, Description, and TodoId](crates/domain/src/todo/values.rs) are newtypes with private fields. Unlike TypeScript type aliases, they're distinct types: a raw string or a description can't stand in for a title. Input length and UUID syntax still need runtime validation. |
| Entity methods mutate the Todo. | Editing takes an exclusive borrow (`&mut self`). A transition that changes the type takes ownership (`self`) and returns the new type. Compile-fail tests cover the two invalid calls above. |
| The domain branches on the state read from storage. | The compiler can't know the contents of a row. The [row conversion](crates/infrastructure/src/row.rs) builds a `StoredTodo` enum; a `match` selects one of its three variants before calling the typed transition. |
| A Protocol lets the application swap repository implementations. | A trait plays a similar role to a Go interface, except the adapter explicitly declares `impl TodoRepository for ...`. Requests share the implementation through `Arc<dyn TodoRepository>`; `async_trait` makes its asynchronous methods callable through this trait object. |

## API

| Method | Path | Operation | Success |
| --- | --- | --- | --- |
| POST | `/todos` | Create | 201 |
| GET | `/todos` | List the latest 20, newest first | 200 |
| GET | `/todos/{id}` | Get one | 200 |
| PUT | `/todos/{id}` | Replace title and description | 200 |
| PATCH | `/todos/{id}/start` | Not started → in progress | 200 |
| PATCH | `/todos/{id}/complete` | In progress → completed | 200 |
| DELETE | `/todos/{id}` | Delete | 204 (no body) |

POST and PUT require a `title` (1–100 characters). The `description` is optional (up to 1,000 characters); omitting it on PUT clears it. Responses use an empty string for a missing description, Unix timestamps in milliseconds, and `null` for `completed_at` until completion.

An invalid transition returns 400, a missing Todo 404, a conflicting write 409, and an invalid UUID or field value 422. After a conflict, read the latest Todo before trying again.

## Verify

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Tests do not touch the development database. Run `cargo test -p domain --doc` for just the compile-fail examples, or read the [HTTP integration tests](crates/server/tests/http.rs) to follow a Todo through persistence and a database reopen.
