# Installor v0.1.0 — Initial Public Release

Installor v0.1.0 establishes a local-first software catalog and installer orchestration workflow with an intentionally explicit trust model.

## Highlights

- Maintain a structured catalog locally in SQLite.
- Review installed software discovered through platform-native sources.
- Import a public Google Sheet from any account without OAuth, an API key, or a Google Cloud project.
- Optionally synchronize changes through a Sheet-owner-deployed Apps Script bridge.
- Download direct installers with progress and optional SHA-256 validation.
- Execute supported package-manager and custom-installer plans sequentially.
- Review and explicitly confirm fuzzy matches for installer files already on disk.

## Windows downloads

- `Installor_0.1.0_x64-setup.exe` — guided NSIS installer.
- `Installor_0.1.0_x64_en-US.msi` — Windows Installer package.
- `installor.exe` — standalone executable for portable evaluation.
- `SHA256SUMS.txt` — integrity values for attached Windows binaries.

## Verification

The Windows application was compiled, launched, and checked for database initialization. The release tree passes frontend tests, Rust tests, formatting, strict Clippy linting, JavaScript syntax checks, and repository/privacy audits.

## Known limitations

- v0.1.0 Windows binaries are unsigned and may trigger Microsoft SmartScreen.
- macOS and Linux code paths have parser/plan tests and native CI configuration but were not manually exercised on real hardware for this release.
- Public Google Sheet import is read-only. Write-back requires the optional Apps Script deployment documented in the repository.
- Installer provenance, licensing, publisher signatures, and source trust remain user or organization responsibilities.

Read the [User Guide](USER_GUIDE.md), [Security Policy](../SECURITY.md), and [Changelog](../CHANGELOG.md) before deployment.
