# Contributing to Installor

Thank you for improving Installor. Contributions are welcome when they preserve explicit user control, local-first data ownership, cross-platform clarity, and truthful documentation.

## Before opening work

- Search existing issues and discussions.
- Use an issue for behavior changes, security-sensitive design, or work spanning multiple modules.
- Never attach real institutional spreadsheets, software inventories, student records, access tokens, credential-vault exports, or local SQLite databases.
- Read the [Code of Conduct](CODE_OF_CONDUCT.md), [Responsible Use Policy](RESPONSIBLE_USE.md), and [Architecture](docs/ARCHITECTURE.md).

## Development setup

Install Node.js 20+, stable Rust, and the [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your operating system.

```powershell
git clone https://github.com/the-sudipta/installor.git
cd installor
npm ci
npm test
cargo test --manifest-path src-tauri/Cargo.toml
npm run dev
```

## Branches and commits

- Branch from `main` using a focused name such as `fix/checksum-message` or `feat/catalog-export`.
- Keep commits reviewable and use an imperative subject; Conventional Commit prefixes are encouraged.
- Do not combine unrelated refactors with a functional change.
- Update user-facing documentation, tests, `CHANGELOG.md`, and citation/version metadata when applicable.

## Required validation

```powershell
npm test
node --check src/main.js
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Run `npm run build` for changes affecting packaging, capabilities, native icons, file access, or Tauri configuration. Describe any platform checks you could not perform—do not mark them as passed.

## Engineering expectations

- Treat downloads and process execution as security boundaries.
- Pass executables and argument arrays directly; do not add shell interpolation.
- Detection and heuristic matches must remain suggestions requiring user review.
- Preserve sequential installation unless the locking and elevation risks are explicitly addressed.
- Store credentials through the operating-system credential service, never SQLite or source files.
- Provide deterministic fixtures using synthetic data; do not copy production or institutional records.
- Keep features usable without Google account integration whenever feasible.

## Pull requests

Complete the pull-request template, link the issue, explain tradeoffs, and include screenshots for visible changes. A maintainer may request narrower scope, tests, security review, accessibility fixes, or platform evidence before merge.

By submitting a contribution, you agree that it is provided under the repository's [Apache License 2.0](LICENSE) and that you have the right to submit it.
