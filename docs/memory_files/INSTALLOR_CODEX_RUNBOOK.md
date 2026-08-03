# Installor - Codex Implementation Runbook

Companion to `INSTALLOR_DOCUMENTATION.md` - read that first for the *why*; this file is the *how*, phase by phase.

## How to use this runbook

- `[CODEX]` = Codex executes this without waiting for input.
- `[MASTER]` = requires Sudipta specifically (a Google account action, a real-hardware test, a judgment call) - Codex should stop, state clearly what's needed, and not attempt to fake or skip it.
- Every phase has a **Definition of Done (DoD)**. Do not start the next phase until the current one's DoD is met.
- Do not hand-type crate/package versions from memory - use `cargo add <name>` / `npm install <name>` so the resolved version is whatever is actually newest-compatible at build time. The versions in `INSTALLOR_DOCUMENTATION.md` 6.1 are a verified floor as of 2026-08-02, not a ceiling.
- If a phase turns out to depend on a decision this runbook didn't anticipate, log it under a new dated entry in the Documentation file's Changelog (14) rather than making a silent judgment call.

---

## Phase 0 - Environment & Scaffolding

**[CODEX]**
```bash
npm create tauri-app@latest installor -- --template vanilla
cd installor && npm install
cd src-tauri
cargo add tauri --features protocol-asset
cargo add serde --features derive
cargo add serde_json
cargo add reqwest --features json,stream
cargo add tokio --features full
cargo add rusqlite --features bundled
cargo add sha2 directories keyring walkdir thiserror strsim
cargo add uuid --features v4,serde
cargo add chrono --features serde
cargo add winreg --target 'cfg(windows)'
cargo add plist --target 'cfg(target_os = "macos")'
cd .. && npm run tauri dev   # confirm the default template window opens
```
Create the folder structure from `INSTALLOR_DOCUMENTATION.md` 7 (empty modules with `mod.rs` stubs are fine at this stage).

**DoD:** `npm run tauri dev` opens a window with no build errors; folder structure matches 7; `git init` + first commit made.

---

## Phase 1 - Data Models & Local SQLite Cache

**[CODEX]** - `src-tauri/src/models.rs`: define `CatalogEntry`, `InstallationEntry`, `DetectedEntry`, `SourceType` (enum: `Winget|Brew|Apt|Dnf|Pacman|Flatpak|Custom`), `InstallerType` (enum: `Msi|Inno|Nsis|Pkg|Dmg|Deb|Rpm|AppImage|Other`) - all `#[derive(Serialize, Deserialize, Clone, Debug)]`, fields matching `INSTALLOR_DOCUMENTATION.md` 10.1 exactly (column names become struct field names, `snake_case`).

`src-tauri/src/db/schema.rs`: SQLite schema mirroring the two Sheet tabs plus two local-only tables:

```sql
CREATE TABLE IF NOT EXISTS catalog (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, publisher TEXT, category TEXT,
    platforms TEXT, source_type TEXT NOT NULL, source_id TEXT, download_url TEXT,
    installer_type TEXT, silent_args TEXT, checksum_sha256 TEXT, tags TEXT, notes TEXT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS installations (
    id TEXT PRIMARY KEY, software_id TEXT NOT NULL REFERENCES catalog(id),
    machine_id TEXT NOT NULL, machine_label TEXT, installed INTEGER NOT NULL DEFAULT 0,
    installed_version TEXT, installed_path TEXT, installer_file_path TEXT,
    last_verified_at TEXT, updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS sync_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT, op_type TEXT NOT NULL,
    payload_json TEXT NOT NULL, created_at TEXT NOT NULL, retries INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
```
Run schema creation on every app start (`CREATE TABLE IF NOT EXISTS` makes this idempotent - no separate migration framework needed at this scale).

**DoD:** unit test opens an in-memory SQLite DB, runs the schema, inserts and reads back one row of each table.

---

## Phase 2 - OS Detection Modules

**[CODEX]** Define one trait, three implementations:

```rust
pub trait SoftwareDetector {
    fn detect(&self) -> Result<Vec<DetectedEntry>, InstallorError>;
}
```

- **`detect/windows.rs`**: enumerate `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*`, `HKLM\SOFTWARE\WOW6432Node\...\Uninstall\*`, and `HKCU\...\Uninstall\*` via `winreg`. Read `DisplayName`, `DisplayVersion`, `Publisher`, `InstallLocation`. **Skip** entries with no `DisplayName`, or where `SystemComponent == 1`, or where `ParentKeyName` is present (these are patches/components, not user-facing software) - without this filter the list will be dominated by noise.
- **`detect/macos.rs`**: shell out to `system_profiler SPApplicationsDataType -json`, parse the `SPApplicationsDataType` array; map `_name`  name, `version`  version, `obtained_from`  publisher (fallback), `path`  installed_path.
- **`detect/linux.rs`**: check binary presence with `which` (or `std::process::Command::new(x).arg("--version")` and check exit code) for `dpkg-query`, `rpm`, `pacman`, `flatpak`, `snap`, in that order, and run only the ones present:
  - `dpkg-query -W -f='${Package}\t${Version}\t${Maintainer}\n'`
  - `rpm -qa --qf '%{NAME}\t%{VERSION}\t%{VENDOR}\n'`
  - `pacman -Q` (name + version only; run `pacman -Qi <name>` only if richer metadata is explicitly requested, to avoid an N+1 shell-out cost on large systems)
  - `flatpak list --app --columns=application,version,origin`
  - `snap list`
  Tag each `DetectedEntry` with which package manager produced it (this becomes the `source_type` suggestion when the user adds it to the catalog).

**[CODEX]** Unit tests use **fixture files** (`tests/fixtures/sample_dpkg_status.txt`, etc.) rather than the real registry/`system_profiler`/package managers, so CI doesn't depend on the runner's actual installed software.

**DoD:** `detect_installed_software()` Tauri command returns a populated `Vec<DetectedEntry>` when run manually on a real dev machine of whichever OS Codex has available; fixture-based unit tests pass on all three parser modules regardless of host OS.

---

## Phase 3 - Google Apps Script Bridge

### 3.1 - `[CODEX]` writes the script to `apps-script/Code.gs` (exact source below - do not modify the contract without updating `INSTALLOR_DOCUMENTATION.md` 10.2 to match):

```javascript
// Installor - Google Apps Script Web App bridge.
// Deploy from Extensions > Apps Script inside your own Google Sheet.
// Set SHARED_SECRET to a value only you know before deploying, then paste the
// same value into Installor's Settings screen along with this deployment's URL.

const SHARED_SECRET = "REPLACE_WITH_YOUR_OWN_SECRET";
const CATALOG_SHEET = "Catalog";
const INSTALLATIONS_SHEET = "Installations";

const CATALOG_COLUMNS = ["id","name","publisher","category","platforms","source_type","source_id",
  "download_url","installer_type","silent_args","checksum_sha256","tags","notes","created_at","updated_at"];
const INSTALLATIONS_COLUMNS = ["id","software_id","machine_id","machine_label","installed",
  "installed_version","installed_path","installer_file_path","last_verified_at","updated_at"];

function doGet(e) { return process_(e, "GET"); }
function doPost(e) { return process_(e, "POST"); }

function process_(e, method) {
  try {
    const params = method === "GET" ? (e.parameter || {}) : JSON.parse(e.postData.contents || "{}");
    if (params.token !== SHARED_SECRET) return jsonOut_({ error: "unauthorized" });

    const action = params.action;
    const ss = SpreadsheetApp.getActiveSpreadsheet();
    ensureSheets_(ss);

    switch (action) {
      case "ping":
        return jsonOut_({
          ok: true,
          sheet_name: ss.getName(),
          catalog_count: readRows_(ss, CATALOG_SHEET, CATALOG_COLUMNS).length,
          installations_count: readRows_(ss, INSTALLATIONS_SHEET, INSTALLATIONS_COLUMNS).length
        });
      case "list":
        return jsonOut_({
          catalog: readRows_(ss, CATALOG_SHEET, CATALOG_COLUMNS),
          installations: readRows_(ss, INSTALLATIONS_SHEET, INSTALLATIONS_COLUMNS)
        });
      case "upsert_catalog":
        return jsonOut_(upsertRow_(ss, CATALOG_SHEET, CATALOG_COLUMNS, params.row));
      case "delete_catalog":
        return jsonOut_(deleteRow_(ss, CATALOG_SHEET, params.id));
      case "upsert_installation":
        return jsonOut_(upsertRow_(ss, INSTALLATIONS_SHEET, INSTALLATIONS_COLUMNS, params.row));
      case "delete_installation":
        return jsonOut_(deleteRow_(ss, INSTALLATIONS_SHEET, params.id));
      default:
        return jsonOut_({ error: "unknown_action" });
    }
  } catch (err) {
    return jsonOut_({ error: String(err) });
  }
}

function ensureSheets_(ss) {
  if (!ss.getSheetByName(CATALOG_SHEET)) ss.insertSheet(CATALOG_SHEET).appendRow(CATALOG_COLUMNS);
  if (!ss.getSheetByName(INSTALLATIONS_SHEET)) ss.insertSheet(INSTALLATIONS_SHEET).appendRow(INSTALLATIONS_COLUMNS);
}

function readRows_(ss, sheetName, columns) {
  const sh = ss.getSheetByName(sheetName);
  const lastRow = sh.getLastRow();
  if (lastRow < 2) return [];
  const values = sh.getRange(2, 1, lastRow - 1, columns.length).getValues();
  return values.filter(r => r[0] !== "").map(r => {
    const obj = {};
    columns.forEach((c, i) => obj[c] = r[i]);
    return obj;
  });
}

function findRowIndexById_(sh, id) {
  const lastRow = sh.getLastRow();
  if (lastRow < 2) return -1;
  const ids = sh.getRange(2, 1, lastRow - 1, 1).getValues();
  for (let i = 0; i < ids.length; i++) if (ids[i][0] === id) return i + 2;
  return -1;
}

function upsertRow_(ss, sheetName, columns, row) {
  const sh = ss.getSheetByName(sheetName);
  const lock = LockService.getScriptLock();
  lock.waitLock(10000);
  try {
    const rowValues = columns.map(c => row[c] !== undefined ? row[c] : "");
    const existingIndex = findRowIndexById_(sh, row.id);
    if (existingIndex === -1) sh.appendRow(rowValues);
    else sh.getRange(existingIndex, 1, 1, columns.length).setValues([rowValues]);
    return { ok: true, id: row.id };
  } finally { lock.releaseLock(); }
}

function deleteRow_(ss, sheetName, id) {
  const sh = ss.getSheetByName(sheetName);
  const lock = LockService.getScriptLock();
  lock.waitLock(10000);
  try {
    const idx = findRowIndexById_(sh, id);
    if (idx !== -1) sh.deleteRow(idx);
    return { ok: true };
  } finally { lock.releaseLock(); }
}

function jsonOut_(obj) {
  return ContentService.createTextOutput(JSON.stringify(obj)).setMimeType(ContentService.MimeType.JSON);
}
```

### 3.2 - `[MASTER]` - one-time deployment (Codex cannot do this; it requires your own Google account):

1. Create a new Google Sheet (any name - the tabs `Catalog`/`Installations` are created automatically on first request).
2. Extensions  Apps Script. Delete the placeholder `Code.gs` content, paste in the script above.
3. Change `SHARED_SECRET` to a value only you know.
4. Deploy  New deployment  type **Web app**  Execute as **Me**  Who has access **Anyone**  Deploy. Authorize when prompted (this authorizes the script to act on your own Sheet - it does not grant Installor's developer anything).
5. Copy the resulting URL (`https://script.google.com/macros/s/.../exec`).
6. In Installor's Settings screen, paste the URL and the shared secret, then click "Test Connection" (calls `?action=ping`).

### 3.3 - `[CODEX]` - Rust sync client, `sync/sheets_client.rs`:

```rust
pub struct SheetsClient { base_url: String, token: String, http: reqwest::Client }

impl SheetsClient {
    pub async fn ping(&self) -> Result<PingResponse, InstallorError> {
        let url = format!("{}?action=ping&token={}", self.base_url, self.token);
        Ok(self.http.get(&url).send().await?.json().await?)
    }
    pub async fn list(&self) -> Result<ListResponse, InstallorError> {
        let url = format!("{}?action=list&token={}", self.base_url, self.token);
        Ok(self.http.get(&url).send().await?.json().await?)
    }
    pub async fn upsert_catalog(&self, row: &CatalogEntry) -> Result<(), InstallorError> {
        self.post_("upsert_catalog", serde_json::json!({"row": row})).await
    }
    // upsert_installation / delete_catalog / delete_installation follow the same pattern
    async fn post_(&self, action: &str, mut body: serde_json::Value) -> Result<(), InstallorError> {
        body["action"] = action.into();
        body["token"] = self.token.clone().into();
        self.http.post(&self.base_url).json(&body).send().await?.error_for_status()?;
        Ok(())
    }
}
```

**DoD:** with a real deployed script (Master must have completed 3.2), `test_apps_script_connection` returns `ok:true` with a real sheet name; a round-trip upsert-then-list shows the new row.

---

## Phase 4 - Frontend UI

**[CODEX]** Vanilla JS, no framework (matches Hub convention). Screens:

- **Main table**: checkbox column, Name / Publisher / Category / Status (? installed here / 	 not installed / installed elsewhere) / Actions. Toolbar: Select All, Select None, + Add Software, ? Download Selected, ? Install Selected, ? Settings.
- **Add/Edit modal**: all `Catalog` fields from 10.1; `source_type`/`installer_type` as dropdowns; conditionally show `download_url` only when `source_type = custom`.
- **Settings panel**: Apps Script URL, shared secret (store via `keyring`, not plaintext - see Phase 8), default download folder, machine label.
- **Detected-software review screen**: after `detect_installed_software()`, show a diff-style list ("found on this machine, not yet in your catalog") with one-click "add to catalog" per row - never auto-adds.

**DoD:** every Tauri command from Phases 1-3 is wired to a visible UI action; `npm run tauri dev` demonstrates add  edit  delete  select-all  (mocked) sync round-trip.

---

## Phase 5 - Download Engine

**[CODEX]** `installer/downloader.rs`: `reqwest` streaming GET to `download_url`, write to the configured download folder (default: platform Downloads dir via `directories` crate, subfolder `Installor/`), emit Tauri progress events (`download-progress` with bytes/total) for the frontend to render a progress bar. After download, if `checksum_sha256` is set on the entry, compute SHA-256 via `sha2` and compare; on mismatch, flag the entry red in the UI rather than silently proceeding to install.

**DoD:** downloading a real small test file (e.g. a public `.txt` or a tiny open-source installer) shows live progress in the UI and lands in the expected folder; a deliberately-wrong checksum is correctly flagged.

---

## Phase 6 - Install Engine

**[CODEX]** Implement `build_install_plan` exactly as specified in `INSTALLOR_DOCUMENTATION.md` 10.3, then an executor:

```rust
pub enum InstallPlan {
    Shell { cmd: String, args: Vec<String> },
    MountAndCopy { dmg_path: String, target_dir: Option<String> },
    MakeExecutableAndLaunch { path: String },
    LaunchInteractive { path: String },
}

pub async fn execute(plan: InstallPlan) -> Result<InstallOutcome, InstallorError> {
    match plan {
        InstallPlan::Shell { cmd, args } => {
            let output = tokio::process::Command::new(&cmd).args(&args).output().await?;
            Ok(InstallOutcome::from_exit(output.status.success(), output.stdout, output.stderr))
        }
        InstallPlan::MakeExecutableAndLaunch { path } => {
            #[cfg(unix)] std::fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o755))?;
            tokio::process::Command::new(&path).spawn()?;
            Ok(InstallOutcome::launched())
        }
        InstallPlan::LaunchInteractive { path } => {
            open::that(&path)?; // `open` crate - opens with OS default handler
            Ok(InstallOutcome::launched())
        }
        InstallPlan::MountAndCopy { dmg_path, target_dir } => {
            // macOS only: hdiutil attach, find the .app, ditto/cp to target_dir or /Applications, hdiutil detach
            install_dmg(&dmg_path, target_dir.as_deref()).await
        }
    }
}
```
Add `open = "5"` via `cargo add open` for the `LaunchInteractive` case. Run each `install_selected(ids)` call sequentially (not in parallel) - concurrent `msiexec`/`apt` invocations commonly deadlock on the same package-manager lock.

**DoD:** unit tests confirm exact argv construction for every `(source_type, installer_type)` pair; on a real dev machine, at least one real winget/brew/apt install and one real MSI or Inno-Setup silent install complete successfully end-to-end.

---

## Phase 7 - Heuristic Installer-File Finder

**[CODEX]** `installer/file_finder.rs`: for each `custom`-type catalog entry with no `installer_file_path` recorded for this machine, scan the configured Downloads folder (non-recursive by default; recursive as an opt-in setting) for files with known installer extensions (`.exe .msi .pkg .dmg .deb .rpm .AppImage`), compute a similarity score between each filename (stripped of extension, version numbers, and separators) and the catalog entry's `name` using `strsim::jaro_winkler`, and surface the top 3 matches above a `0.6` threshold per unmatched entry as **suggestions only** - the UI must require explicit user confirmation before writing an `installer_file_path`. Never auto-link.

**DoD:** unit test with a fixture list of filenames (e.g. `"OBS-Studio-30.1-Full-Installer-x64.exe"` against catalog entry `"OBS Studio"`) confirms it ranks above unrelated filenames.

---

## Phase 8 - Sync & Merge Logic

**[CODEX]** `sync/merge.rs`: on `sync_now()`, pull `list()` from the bridge, for each remote row compare `updated_at` against the local cache row of the same `id`; remote wins if strictly newer, local wins otherwise (ties favor local, since it just changed). Any local row not yet pushed (present in `sync_queue`) is pushed after the pull-merge completes, then removed from the queue on success. On network failure, leave the queue intact and retry on the next `sync_now()` (called automatically on app launch and manually via a "Sync Now" button - no background polling in v1, consistent with the no-persistent-service decision in `INSTALLOR_DOCUMENTATION.md` 9.3).

Store the shared secret via the `keyring` crate (OS credential store), not in the SQLite `settings` table in plaintext - the Apps Script URL itself can live in `settings` since it's not sensitive on its own.

**DoD:** integration test using `wiremock` simulates a remote row newer than local (remote wins), a local row newer than remote (local wins, and is pushed), and an offline push (queued, then flushed on the next successful sync).

---

## Phase 9 - Testing

**[CODEX]** Run and confirm passing, don't just write and assume:
- `cargo test` - all unit + integration tests from Phases 1-8.
- Frontend jsdom smoke tests for select-all/none, form validation, table rendering.

**[MASTER]** Manual QA checklist (cannot be automated in this sandbox - needs real OS environments):
- [ ] Real Windows machine: detect installed software, add one custom MSI and one custom Inno-Setup app, download + silently install both to a custom directory, confirm actual install location.
- [ ] Real macOS machine: `system_profiler` detection returns a sane list; install a `.dmg`-packaged app via Installor.
- [ ] Real Linux machine (whichever distro is available): confirm the correct package manager is auto-detected; install one package via the native-manager path.
- [ ] Deploy the Apps Script to a real Sheet (Phase 3.2) and confirm a full add  sync  edit-on-Sheet-directly  re-sync round trip.
- [ ] Confirm SmartScreen/Gatekeeper warnings appear as expected on an unsigned build (so the README caveat is verified, not assumed).

**DoD:** all automated tests green; manual checklist completed and results logged in the Documentation file's Changelog.

---

## Phase 10 - Packaging & CI/CD

**[CODEX]** `.github/workflows/release.yml`: matrix build on `windows-latest`, `macos-latest`, `ubuntu-latest`, using `tauri-apps/tauri-action`, triggered on `v*` tag push, publishing artifacts to a GitHub Release (draft, for Master to review before publishing).

**DoD:** a test tag push produces three platform installers attached to a draft release.

---

## Phase 11 - Documentation Finalization & Master Index

**[MASTER]** After Phases 0-10 are actually complete (not before): update `INSTALLOR_DOCUMENTATION.md` 12 (Deployment) with the real live status and 14 (Changelog) with a new dated entry summarizing what Codex built and what the manual QA found. Then paste the Master Index row below into this Project's `MASTER_INDEX.md` (currently an empty placeholder - this becomes the first real entry and sets the format for every entry after it):

```markdown
# Master Development Hub - Master Index

| Tool | Description | Status | Stack | Docs |
|---|---|---|---|---|
| **Installor** | Cross-platform personal software catalog + installer orchestrator, synced via a self-deployed Google Apps Script bridge to the user's own Sheet | Planning complete, implementation pending | Rust + Tauri v2, vanilla JS, SQLite, Google Apps Script | [INSTALLOR_DOCUMENTATION.md](./INSTALLOR_DOCUMENTATION.md) ú [INSTALLOR_CODEX_RUNBOOK.md](./INSTALLOR_CODEX_RUNBOOK.md) |
```
