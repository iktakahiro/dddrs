---
name: change-todo-use-case
description: dddrs の Todo の振る舞いや HTTP 操作を追加・変更するときに、層の境界を保ち、適切なテストを選ぶ手順。
---

# Todo の操作を変更する

1. 対象操作を README の API 表で確認する。dddpy と揃える項目（状態・ステータスコード・文字数・一覧順）と、Rust の型で表せる条件を切り分ける。
2. 業務ルールは `crates/domain` に置く。値の検証は newtype の生成時に行う。編集は `&mut self`、型の変わる遷移は `self` を消費するメソッドにする。新たに禁止した操作には `compile_fail` の例を付ける。
3. `application` には読み出し・ドメイン操作・保存の順序だけを書く。既存のリポジトリ操作で足りるなら trait を増やさない。永続化を変える場合は `infrastructure` の行変換とマイグレーションをそろえ、版番号による競合検出を保つ。
4. 入力・出力・HTTP エラーは `presentation` で変換する。具体的なリポジトリの組み立ては `server` に限定する。
5. 正常系と失敗時に保存されないことを確認する。層内のテストに加え、HTTP の契約が変わる場合は `crates/server/tests/http.rs` と README の例を更新する。
6. `cargo fmt --all --check`、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`cargo test --workspace --locked` を実行する。依存方向のテストが落ちたら、許可リストを広げる前に責務の置き場所を見直す。
