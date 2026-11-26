# dddrs

[English](README.md) | 日本語

[dddpy](https://github.com/iktakahiro/dddpy) の Todo API を Rust・axum・SQLite で実装した、DDD とオニオンアーキテクチャのサンプル。題材は「未開始の Todo は完了できない」というルール。Rust では、状態が分かっているときは**コンパイル時**に、不明なときは**読み出し後**にこのルールを守れる。

## 動かす

[Rust の stable ツールチェーン](https://rustup.rs/) と C コンパイラがあれば、SQLite サーバーや sqlx-cli のインストールは不要。

```sh
cargo run --locked -p server
```

起動時にカレントディレクトリに `todos.db` を作り、マイグレーションを適用する。接続先は `DATABASE_URL`（既定: `sqlite://todos.db`）、待受先は `BIND_ADDR`（既定: `127.0.0.1:3000`）で変えられる。

別のターミナルで以下を実行する（ID の取り出しに `jq` を使用）。

```sh
base=http://127.0.0.1:3000/todos
id=$(curl -fsS "$base" -H 'Content-Type: application/json' \
  -d '{"title":"夕飯の買い物","description":"野菜を買う"}' | jq -r .id)
curl -sS -i -X PATCH "$base/$id/complete"  # まだ開始していないので 400
curl -fsS -X PATCH "$base/$id/start"       # in_progress
curl -fsS -X PATCH "$base/$id/complete"    # completed
curl -fsS "$base/$id"
```

## 1リクエストを追う

たとえば `PATCH /todos/{id}/start` は次の順に進む。**実行時の呼び出し順と、クレートの依存方向は異なる。**

1. [presentation](crates/presentation/src/lib.rs) が HTTP を受け、[extractor](crates/presentation/src/extract.rs) がパスを `TodoId` に変換する。
2. [application](crates/application/src/lib.rs) の `TodoService::start` が [リポジトリの trait](crates/domain/src/repository.rs) 経由で Todo を読み、[domain](crates/domain/src/todo.rs) に状態遷移を依頼する。
3. [infrastructure](crates/infrastructure/src/lib.rs) が同じ trait の実装として、sqlx で変更を保存する。読み出し時の版番号を更新条件に含めるので、後から来た古い更新が完了状態を巻き戻すことはない。
4. presentation が結果を JSON に変換する。どの実装を使うかは、起動時に [server](crates/server/src/main.rs) が一度だけ決める。

| クレート | 直接依存するワークスペース内のクレート |
| --- | --- |
| domain | なし |
| application | domain |
| infrastructure | domain |
| presentation | application、domain |
| server（起動・配線） | presentation、application、infrastructure |

クレートは Go のパッケージのように `use` で参照するが、別クレートを使うには自身の `Cargo.toml` に依存宣言が必要。たとえば application から `infrastructure::SqliteTodoRepository` を参照してもコンパイルできない。マニフェストに依存を書き足して境界を緩めた場合も、[依存方向のテスト](crates/server/tests/layers.rs) が検出する。

## Rust が何を保証するか

```rust
use domain::{Title, Todo};

let title = Title::try_from(String::from("夕飯の買い物")).unwrap();
let pending = Todo::new(title, None);   // Todo<NotStarted>
// pending.complete();                 // この型には complete がない
let active = pending.start();          // 所有権を渡して Todo<InProgress> を受け取る
// pending.start();                    // 以前の値はもう使えない
let done = active.complete();          // Todo<Completed> に完了時刻が入る
```

| dddpy の表現 | dddrs の表現とその理由 |
| --- | --- |
| 値オブジェクトは frozen dataclass | [Title・Description・TodoId](crates/domain/src/values.rs) は非公開フィールドを持つ newtype。TypeScript の型エイリアスとは違い、文字列や別の値オブジェクトをそのまま渡せない。長さや UUID の形式は入力時に検証する。 |
| エンティティのメソッドが状態を書き換える | 内容の編集は `&mut self`（排他的な借用）。状態の型が変わる遷移は `self` を消費して新しい型を返す。上のコメントアウトした誤用は `compile_fail` テストで確認できる。 |
| DB の値を読み、状態を条件分岐で検査 | DB の行や HTTP 入力までコンパイラが保証するわけではない。[行からの復元](crates/infrastructure/src/row.rs)で3状態を表す enum `StoredTodo` を作り、`match` で分岐してから型付きの遷移に委譲する。 |
| Protocol でリポジトリを差し替える | trait は Go の interface に近いが、実装側で `impl TodoRepository for ...` を宣言する。`Arc<dyn TodoRepository>` は複数リクエストで共有できる trait オブジェクト。`async_trait` により非同期メソッドもこの境界から呼べる。 |

## API

| メソッド | パス | 内容 | 成功 |
| --- | --- | --- | --- |
| POST | `/todos` | 作成 | 201 |
| GET | `/todos` | 作成日時の降順、最新20件 | 200 |
| GET | `/todos/{id}` | 1件取得 | 200 |
| PUT | `/todos/{id}` | タイトル・説明を置換 | 200 |
| PATCH | `/todos/{id}/start` | 未開始 → 進行中 | 200 |
| PATCH | `/todos/{id}/complete` | 進行中 → 完了 | 200 |
| DELETE | `/todos/{id}` | 削除 | 204（本文なし） |

POST・PUT の本文には `title`（1〜100文字）が必須。`description`（最大1,000文字）は省略可能で、PUT で省略すると説明を消す。レスポンスでは説明なしを空文字、時刻を Unix エポックからのミリ秒、未完了の `completed_at` を `null` で返す。

不正な状態遷移は 400、対象が存在しないときは 404、並行更新の競合は 409、UUID や値の検証エラーは 422。競合時は最新の Todo を読み直す。

## 検証

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

テストは開発用 DB を使わない。状態遷移の型エラーだけを試すには `cargo test -p domain --doc`、HTTP から SQLite への保存と再接続を追うには [HTTP 統合テスト](crates/server/tests/http.rs) を参照。
