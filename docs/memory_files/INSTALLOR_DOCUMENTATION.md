# Installor — Product and Architecture Documentation

Project folder/name: **installor**. Product name: **Installor**.

## 1. Purpose

Installor is a local-first desktop application for maintaining a portable software catalog, seeing what is installed on each machine, acquiring trusted installers, and running repeatable installations across Windows, macOS, and Linux.

## 2. Non-goals and boundaries

- It is not a background service and does not poll continuously.
- It never silently adds detected software or links guessed installer files.
- It does not bypass operating-system elevation, SmartScreen, Gatekeeper, or package-manager policies.
- A public Google Sheet link provides anonymous **read-only** access. Google does not expose anonymous write access to shared Sheets; write-back therefore uses the optional self-deployed Apps Script bridge.

## 3. User flows

1. Add/edit a catalog entry manually, import a public Sheet, or review detected software.
2. Select applications and download them with live byte progress.
3. Verify SHA-256 when supplied; a mismatch is surfaced and not marked ready.
4. Confirm any heuristic installer-file suggestion.
5. Install selections sequentially and inspect success/error outcomes.
6. Optionally synchronize changes to a private Sheet through Apps Script.

## 4. Google Sheet modes

### Public-link import (recommended, zero setup)

Any owner can set a Sheet to **Anyone with the link → Viewer**. Installor extracts the spreadsheet ID from the normal Share URL, requests the `Catalog` tab through Google's CSV visualization endpoint, and optionally imports `Installations`. There is no OAuth, API key, Google Cloud project, or account connection. Required catalog header: `name`; all supported headers are in `docs/CATALOG_TEMPLATE.csv`. Missing IDs are deterministically derived from the name.

### Optional bidirectional bridge

`apps-script/Code.gs` is deployed from the owner Sheet. A shared secret authorizes `ping`, `list`, upsert, and delete actions. The secret is stored through Rust `keyring`; only the non-sensitive deployment URL is stored in SQLite. Pull-merge compares ISO `updated_at`; strictly newer remote values win, ties remain local. Pending local operations are queued and retried after a later successful pull.

## 5. Security model

- Local application state: per-user app-data `installor.sqlite3`.
- Bridge credential: OS credential store (`com.installor.desktop` service).
- HTTP: reqwest TLS, status validation, structured API errors.
- Installer integrity: optional SHA-256.
- Command safety: executable plus argv arrays; no shell interpolation. Optional silent arguments are parsed as words.
- Destructive actions require explicit UI actions; installs show a confirmation and execute one at a time.

## 6. Technology

Tauri v2, Rust 2024, vanilla ES modules, SQLite/rusqlite, reqwest/tokio, keyring, winreg, serde, sha2, strsim, Vitest/jsdom, and Google Apps Script.

## 7. Source layout

```text
src/                         desktop UI and jsdom-tested UI helpers
src-tauri/src/models.rs      shared serialized models
src-tauri/src/db/            SQLite schema and data access
src-tauri/src/detect/        Windows/macOS/Linux detection and parsers
src-tauri/src/sync/          public import, Apps Script client, merge queue
src-tauri/src/installer/     downloads, plans, execution, file matching
apps-script/                 optional write-back bridge
docs/                        templates and project memory
.github/workflows/           three-platform release automation
```

## 8. Local data

Tables: `catalog`, `installations`, `sync_queue`, and `settings`. Schema creation is idempotent on each start. Foreign keys cascade installation removal when a catalog entry is deleted.

## 9. Detection

- Windows: 64-bit and 32-bit HKLM plus HKCU uninstall registries; entries missing `DisplayName`, system components, and child patches are filtered.
- macOS: `system_profiler SPApplicationsDataType -json`.
- Linux: present package tools among dpkg, rpm, pacman, flatpak, and snap.

Registry/macOS discoveries default to `custom` because their origin does not prove a package-manager identifier. Linux records retain the actual manager source.

## 10. Data contracts

### 10.1 Sheet columns

Catalog: `id,name,publisher,category,platforms,source_type,source_id,download_url,installer_type,silent_args,checksum_sha256,tags,notes,created_at,updated_at`.

Installations: `id,software_id,machine_id,machine_label,installed,installed_version,installed_path,installer_file_path,last_verified_at,updated_at`.

Source types: `winget`, `brew`, `apt`, `dnf`, `pacman`, `flatpak`, `custom`. Installer types: `msi`, `inno`, `nsis`, `pkg`, `dmg`, `deb`, `rpm`, `appimage`, `other`.

### 10.2 Bridge actions

GET: `ping`, `list`. POST JSON: `upsert_catalog`, `delete_catalog`, `upsert_installation`, `delete_installation`. Every request includes `token`.

### 10.3 Installation plan mapping

- Native sources call their manager directly with non-interactive agreement flags where supported.
- MSI uses `msiexec /i FILE /qn /norestart`; Inno and NSIS use their standard silent switches; explicit `silent_args` augment/override defaults as appropriate.
- PKG uses macOS `installer`; DMG mounts, copies its `.app`, then detaches; DEB/RPM use native tools; AppImage becomes executable and launches; unknown installers open interactively.

## 11. Verification

Automated coverage includes schema round trips, all parser families, actual Windows registry detection, public-link parsing/CSV import, install argv construction, all custom installer variants, fuzzy file ranking, remote/local timestamp merge, failed push retention and successful retry, UI select/clear, validation, search, rendering status, and tagged CI builds.

Commands:

```powershell
npm test
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
```

## 12. Deployment

`npm run build` creates the current-platform signed/unsigned Tauri bundle in `src-tauri/target/release/bundle`. Pushing a `v*` tag runs tests and creates Windows, macOS, and Ubuntu artifacts in a draft GitHub Release. Production distributors should configure platform signing; unsigned builds legitimately trigger SmartScreen/Gatekeeper warnings.

## 13. Manual release checklist

- Validate a custom MSI/Inno install on a disposable Windows VM.
- Validate a DMG copy on macOS and native package install on a Linux VM.
- Confirm public import against an owner-independent Sheet.
- If write-back is desired, deploy `Code.gs` and verify add → sync → Sheet edit → sync.
- Review unsigned warning behavior or configure signing.

## 14. Changelog

### 2026-08-03

- Finalized the Installor product identity throughout the application, source, storage paths, credential service, documentation, release workflow, and installers.
- Integrated the approved package-prism installation-arrow logo and generated native Windows, macOS, Linux, iOS, and Android icon assets from the production master.
- Built the complete Tauri application from an empty workspace.
- Added the account-free public Google Sheet import requested during implementation, while preserving the optional bridge for write-back.
- Added cross-platform detection, local cache, downloads/checksums, install execution, file suggestions, merge/retry, polished UI, automated tests, templates, and release CI.
- Windows compilation and automated verification completed locally; macOS/Linux packaging is encoded in CI and requires those runners/hardware for final platform-specific manual QA.
