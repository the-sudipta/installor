# Changelog

All notable changes to Installor are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Planned

- Complete signed-package and real-hardware QA on Windows, macOS, and representative Linux distributions.
- Add richer provenance and policy metadata without weakening explicit user control.

## [0.2.0] — 2026-08-06

### Added

- In-app, version-matched Google Apps Script setup with a generated credential-vault secret and one-click script copying.
- A guided write-back wizard that saves, tests, and reports the connected Google Sheet before offering a full-library push.
- Bulk checkboxes for detected software, including Select visible, Clear, Add selected, and Add all actions.
- Bulk catalog removal with explicit confirmation that applications and files are not uninstalled or deleted.
- Detailed synchronization reports covering uploaded, downloaded, removed, and pending records.
- User-defined device categories and installation-detail dialogs in the catalog.
- Contextual information controls throughout catalog, detection, setup, synchronization, and local preferences.
- Remote deletion tombstones for catalog and installation records.

### Changed

- Existing local records can now be queued and pushed after Google synchronization is configured, eliminating the v0.1 backfill gap.
- Sync state now displays connection, pending-change, and last-success information instead of a generic completion toast.
- Google Sheet setup and the named-device → Sheet → named-device workflow are documented directly inside Installor.

### Fixed

- Pending local removals cannot be resurrected by an older remote list during synchronization.
- Full-library pushes deduplicate queued upserts while preserving pending deletions.

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

[Unreleased]: https://github.com/the-sudipta/installor/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/the-sudipta/installor/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/the-sudipta/installor/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/the-sudipta/installor/releases/tag/v0.1.0
