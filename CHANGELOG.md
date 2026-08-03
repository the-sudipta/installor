# Changelog

All notable changes to Installor are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Planned

- Complete signed-package and real-hardware QA on Windows, macOS, and representative Linux distributions.
- Add richer provenance and policy metadata without weakening explicit user control.

## [0.1.1] — 2026-08-03

### Fixed

- Made detector implementations compile cleanly under strict Clippy checks on their native Windows, macOS, and Linux runners.
- Removed unreachable platform stubs while preserving cross-platform parser coverage.
- Switched the public project website to the repository's dedicated GitHub Pages workflow.

## [0.1.0] — 2026-08-03

### Added

- Local-first software catalog backed by SQLite.
- Windows, macOS, and Linux installed-software detection modules.
- Manual catalog management and review-before-import detection workflow.
- Account-independent, read-only import from a public Google Sheet.
- Optional self-deployed Google Apps Script bridge with credential-vault secret storage.
- Streaming installer downloads, progress events, and optional SHA-256 verification.
- Sequential installation plans for native package managers and common installer formats.
- Explicitly confirmed fuzzy matching for installer files already on disk.
- Timestamp-based local/remote reconciliation with an offline retry queue.
- Tauri desktop interface, native Installor icon set, automated tests, CI, release automation, and publication documentation.

### Verified

- Windows production build, executable launch, SQLite initialization, Rust test suite, frontend test suite, formatting, and strict linting.
- macOS and Linux parsers and plan construction are exercised by platform-independent tests; native packaging and real-hardware QA remain release limitations.

[Unreleased]: https://github.com/the-sudipta/installor/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/the-sudipta/installor/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/the-sudipta/installor/releases/tag/v0.1.0
