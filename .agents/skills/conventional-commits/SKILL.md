---
name: conventional-commits
description: Use when creating git commits, splitting changes into logical commits, or writing pull request titles and descriptions in dddrs. Enforces Conventional Commits v1.0.0, English-only messages, clean commit splitting, and pre-commit verification gates.
---

# Conventional Commits & PR Hygiene

Follow [Conventional Commits v1.0.0](https://www.conventionalcommits.org/en/v1.0.0/) for all git commit messages and pull request titles in this repository.

## 1. Format

```text
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### Allowed Types
- `feat`: New user-facing capability, API endpoint, or domain feature.
- `fix`: Bug fix (including concurrency, persistence, or error-mapping fixes).
- `refactor`: Code restructuring without changing external behavior.
- `docs`: Documentation changes (`README.md`, `README.ja_JP.md`, `AGENTS.md`, skills, comments).
- `test`: Adding or improving tests without changing production code.
- `chore`: Maintenance, dependency updates, Cargo configuration (`.cargo/config.toml`, `Cargo.lock`).
- `ci`: GitHub Actions workflows and CI scripts (`.github/workflows/`).
- `perf`: Performance improvements.
- `build`: Build system or workspace manifest changes.

### Recommended Scopes
Use a workspace crate or subsystem name when changes are localized; omit the scope for cross-cutting changes spanning multiple layers:
- `domain`, `application`, `infrastructure`, `presentation`, `server`
- `cargo`, `ci`, `docs`, `skills`, `e2e`

### Style Rules
- **English only** for all commit subjects, commit bodies, PR titles, and PR descriptions (never include Japanese text or bot banners).
- Use the **imperative mood** in the subject (`add`, `fix`, `remove`, `use` — not `added`, `fixes`, `removing`).
- Start the description with a lowercase letter and omit a trailing period.
- Keep the subject line concise (aim for ≤ 72 characters).
- Add a blank line before the body when explaining **why** the change was made or summarizing key architectural impacts.
- Mark breaking changes with `!` after the type/scope (e.g., `feat(presentation)!: ...`) and a `BREAKING CHANGE:` footer.

## 2. Examples

Good:
```text
feat(domain): enforce Todo lifecycle transitions in TodoState ADT
fix(infrastructure): use BEGIN IMMEDIATE and map SQLITE_BUSY_SNAPSHOT to Conflict
refactor(domain): remove redundant TodoStatus enum and unify on TodoState
chore(cargo): configure 7-day dependency publish-age cooldown
docs: add layer-by-layer design guide with real code examples
test(server): add concurrent WAL HTTP start test
```

Avoid:
- `update files` (missing type, vague)
- `fix: Fixed a bug.` (past tense, capitalized, trailing period)
- Non-English subjects, bodies, or PR descriptions (always write commits and PRs in English)
- Mixing unrelated changes (e.g., feature code + Cargo cooldown + README rewrite) into one commit.

## 3. Commit Splitting & Pre-Commit Checklist

1. **Split by concern:** Keep production code changes, standalone test additions, dependency/tooling updates, and unrelated documentation rewrites in separate commits (or separate PRs).
2. **Inspect staged files:** Run `git status` and `git diff --cached` before committing:
   - Never stage generated artifacts (`todos.db`, `todos.db-wal`, `todos.db-shm`, `target/`).
   - Check that no unintended Japanese text leaked outside `README.ja_JP.md` (and the language-switch link in `README.md`).
3. **Run quality gates before committing:**
   ```sh
   cargo fmt --all --check
   cargo clippy --workspace --all-targets --locked -- -D warnings
   cargo test --workspace --locked
   ```
4. **Pull requests:** Use a Conventional Commit style PR title in English and a structured English summary (`## Summary` / `### Changes`). Never push directly to `main`; work on a descriptive feature branch (`feat/...`, `fix/...`, `refactor/...`, `docs/...`, `chore/...`).
