# Repository Guidelines

## Project Structure & Module Organization

This repository builds the Rust CLI binary `pm`. `src/main.rs` is the executable entry point, while `src/lib.rs` parses commands and opens the database. CLI argument types live in `src/cli.rs`; command handlers are grouped under `src/commands/`; SQLite access and migrations are implemented in `src/db/` and `migrations/`; Markdown parsing and template expansion live in `src/prompt/`. Unit tests stay beside their modules under `#[cfg(test)]`. End-to-end CLI behavior belongs in `tests/cli.rs`. Build artifacts are written to `target/`.

## Build, Test, and Development Commands

- `make build`: compile all development targets with the locked dependency graph.
- `cargo run -- <command>`: run `pm` locally, for example `cargo run -- list`.
- `make fmt`: format all Rust code with `rustfmt`.
- `make lint`: run Clippy for all targets and treat warnings as errors.
- `make test`: run unit and CLI integration tests.
- `make check`: run formatting checks, Clippy, and the complete test suite; use this before committing.
- `make build-release`: produce the optimized binary at `target/release/pm`.

## Coding Style & Naming Conventions

Follow standard Rust formatting and four-space indentation; do not hand-align code against `rustfmt`. Use `snake_case` for modules, functions, variables, and test names; use `UpperCamelCase` for structs and enums. Keep command parsing in `cli.rs`, behavior in the matching command module, and persistence logic in `db/`. Preserve the Pipe-first contract: stdout contains command data, while diagnostics and prompts go to stderr.

## Testing Guidelines

Use ordinary Rust `#[test]` functions for focused logic. Use `assert_cmd`, `predicates`, and `tempfile` in `tests/cli.rs` for observable CLI behavior and isolated databases. Name tests after behavior, such as `gets_prompt_by_id`. There is no numeric coverage threshold; every behavior change or bug fix should include a regression test. Add new schema changes as sequential files under `migrations/` and test both fresh and upgraded databases.

## Commit & Pull Request Guidelines

Use Conventional Commits, matching repository history: `feat: add ...`, `fix: prevent ...`, or `refactor: simplify ...`. Keep the summary imperative, lowercase, and under 72 characters. Develop features and fixes on focused branches based on `master`; do not force-push. Pull requests should explain user-visible CLI changes, data or migration impact, and validation performed. Link the relevant issue when available. Screenshots are unnecessary for this CLI; include concise terminal examples when output or arguments change.
