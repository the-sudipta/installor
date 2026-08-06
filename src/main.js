import { catalogStatus, detectedKey, matchesSearch, matchesStatus, selectAllIds, validateCatalog } from "./ui.js";

const invoke = (...args) => window.__TAURI__.core.invoke(...args);
const emptyBundle = () => ({ catalog: [], installations: [], deleted_catalog: [], deleted_installations: [] });
const state = {
  bundle: emptyBundle(), settings: {}, sync: { configured: false, pending_changes: 0, last_synced_at: "" },
  selected: new Set(), detectedSelected: new Set(), detected: [], query: "", detectedQuery: "", statusFilter: "all",
};
const $ = (selector) => document.querySelector(selector);
const $$ = (selector) => [...document.querySelectorAll(selector)];

function toast(message, type = "success") {
  const node = document.createElement("div");
  node.className = `toast ${type}`;
  node.textContent = message;
  $("#toast-region").append(node);
  setTimeout(() => node.remove(), 5000);
}

async function call(command, args = {}) {
  try { return await invoke(command, args); }
  catch (error) { toast(String(error), "error"); throw error; }
}

function initials(name = "?") { return name.split(/\s+/).filter(Boolean).slice(0, 2).map((part) => part[0]).join("").toUpperCase() || "?"; }
function selectedIds() { return [...state.selected]; }
function formatTime(value) { if (!value) return "Never synchronized"; const date = new Date(value); return Number.isNaN(date.valueOf()) ? value : `Last sync ${date.toLocaleString()}`; }

function setView(view) {
  $$(".view").forEach((node) => node.classList.toggle("active", node.id === `${view}-view`));
  $$(".nav-item").forEach((button) => button.classList.toggle("active", button.dataset.view === view));
  const titles = { catalog: "Your catalog", detected: "Detected software", settings: "Settings & sync" };
  $("#view-title").textContent = titles[view];
  $("#add-button").classList.toggle("hidden", view !== "catalog");
}

function currentInstall(entry) {
  return state.bundle.installations.find((row) => row.software_id === entry.id && row.machine_label === state.settings.machine_label);
}

function visibleCatalog() {
  return state.bundle.catalog.filter((entry) => matchesSearch(entry, state.query) && matchesStatus(entry, state.bundle.installations, state.settings.machine_label, state.statusFilter));
}

function rebuildDeviceFilter() {
  const select = $("#status-filter");
  const deviceNames = [...new Set(state.bundle.installations.filter((row) => row.installed && row.machine_label).map((row) => row.machine_label))].sort();
  select.querySelector("optgroup")?.remove();
  if (deviceNames.length) {
    const group = document.createElement("optgroup"); group.label = "Device categories";
    for (const name of deviceNames) {
      const option = document.createElement("option"); option.value = `device:${name}`; option.textContent = name; group.append(option);
    }
    select.append(group);
  }
  if (state.statusFilter.startsWith("device:") && !deviceNames.includes(state.statusFilter.slice("device:".length))) state.statusFilter = "all";
  select.value = state.statusFilter;
}

function showMachineDetails(entry, status) {
  $("#machine-dialog-title").textContent = entry.name;
  const list = $("#machine-list");
  list.replaceChildren();
  if (!status.installations.length) {
    const empty = document.createElement("p"); empty.className = "dialog-copy"; empty.textContent = "No connected device has reported this application as installed."; list.append(empty);
  }
  for (const installation of status.installations) {
    const item = document.createElement("article");
    item.className = "machine-item";
    const heading = document.createElement("strong"); heading.textContent = installation.machine_label || "Unnamed device";
    const details = document.createElement("small");
    details.textContent = [installation.installed_version && `Version ${installation.installed_version}`, installation.installed_path, installation.last_verified_at && `Verified ${formatTime(installation.last_verified_at).replace("Last sync ", "")}`].filter(Boolean).join(" · ") || "Installed";
    item.append(heading, details); list.append(item);
  }
  $("#machine-dialog").showModal();
}

function renderCatalog() {
  rebuildDeviceFilter();
  const entries = visibleCatalog();
  const body = $("#catalog-body"); body.replaceChildren();
  for (const entry of entries) {
    const status = catalogStatus(entry, state.bundle.installations, state.settings.machine_label);
    const installer = currentInstall(entry)?.installer_file_path;
    const row = document.createElement("tr"); row.dataset.id = entry.id;
    row.innerHTML = `<td><input class="row-check" type="checkbox"></td><td><strong></strong><small></small></td><td></td><td></td><td><button class="status status-button"></button></td><td><strong></strong><small></small></td><td class="align-right"><div class="row-actions"><button class="icon-button edit" title="Edit software" aria-label="Edit software">✎</button><button class="icon-button danger delete" title="Remove from catalog" aria-label="Remove from catalog">×</button></div></td>`;
    const cells = row.children;
    const checkbox = row.querySelector(".row-check"); checkbox.checked = state.selected.has(entry.id); checkbox.setAttribute("aria-label", `Select ${entry.name}`);
    cells[1].querySelector("strong").textContent = entry.name; cells[1].querySelector("small").textContent = entry.tags || installer || "";
    cells[2].textContent = entry.publisher || "—"; cells[3].textContent = entry.category || "—";
    const statusButton = row.querySelector(".status-button"); statusButton.classList.add(status.key); statusButton.textContent = status.label; statusButton.title = "Show installation details";
    cells[5].querySelector("strong").textContent = entry.source_type; cells[5].querySelector("small").textContent = entry.source_id || entry.installer_type;
    checkbox.addEventListener("change", (event) => { event.target.checked ? state.selected.add(entry.id) : state.selected.delete(entry.id); updateCatalogSelection(); });
    statusButton.addEventListener("click", () => showMachineDetails(entry, status));
    row.querySelector(".edit").addEventListener("click", () => openSoftware(entry));
    row.querySelector(".delete").addEventListener("click", async () => {
      if (!confirm(`Remove ${entry.name} from the Installor catalog? This will not uninstall it or delete files.`)) return;
      await call("delete_catalog", { id: entry.id }); state.selected.delete(entry.id); await refresh();
      toast(state.sync.configured ? `${entry.name} removed; synchronize to update other devices` : `${entry.name} removed from the local catalog`);
    });
    body.append(row);
  }
  $("#empty-catalog").classList.toggle("hidden", state.bundle.catalog.length !== 0);
  $("#catalog-count").textContent = state.bundle.catalog.length;
  $("#installed-count").textContent = state.bundle.catalog.filter((entry) => catalogStatus(entry, state.bundle.installations, state.settings.machine_label).key === "here").length;
  $("#ready-count").textContent = state.bundle.catalog.filter((entry) => entry.source_type !== "custom" ? entry.source_id : entry.download_url || currentInstall(entry)?.installer_file_path).length;
  updateCatalogSelection(entries);
}

function updateCatalogSelection(entries = visibleCatalog()) {
  const count = state.selected.size;
  ["#download-selected", "#install-selected", "#remove-selected"].forEach((id) => { $(id).disabled = count === 0; });
  const visibleIds = entries.map((entry) => entry.id);
  $("#select-all-checkbox").checked = visibleIds.length > 0 && visibleIds.every((id) => state.selected.has(id));
  $("#select-all-checkbox").indeterminate = visibleIds.some((id) => state.selected.has(id)) && !$("#select-all-checkbox").checked;
  const summary = $("#selection-summary"); summary.classList.toggle("hidden", count === 0);
  summary.textContent = count ? `${count} catalog item${count === 1 ? "" : "s"} selected. Actions affect only these records.` : "";
}

async function refresh() { state.bundle = await call("get_catalog"); renderCatalog(); }

function openSoftware(entry = null) {
  const form = $("#software-form"); form.reset(); $("#dialog-title").textContent = entry ? "Edit software" : "Add software";
  if (entry) Object.entries(entry).forEach(([key, value]) => { if (form.elements[key]) form.elements[key].value = value ?? ""; });
  toggleDownloadField(); $("#software-dialog").showModal();
}
function formObject(form) { return Object.fromEntries(new FormData(form).entries()); }
function toggleDownloadField() { $("#download-url-field").classList.toggle("hidden", $("#software-form").elements.source_type.value !== "custom"); }

async function loadSettings() {
  state.settings = await call("get_settings");
  const form = $("#settings-form");
  for (const [key, value] of Object.entries(state.settings)) {
    if (!form.elements[key]) continue;
    if (form.elements[key].type === "checkbox") form.elements[key].checked = value; else form.elements[key].value = value ?? "";
  }
  $("#secret-status").textContent = state.settings.has_shared_secret ? "A secret is stored securely; leave blank to keep it." : "No shared secret is stored yet.";
}

async function loadSyncState() {
  state.sync = await call("get_sync_state"); updateSyncUi();
}

function updateSyncUi() {
  const pending = state.sync.pending_changes || 0;
  $("#sync-label").textContent = state.sync.configured ? "Sync with Sheet" : "Set up sync";
  $("#sync-status").textContent = state.sync.configured ? (pending ? `${pending} change${pending === 1 ? "" : "s"} waiting` : formatTime(state.sync.last_synced_at)) : "Google sync not connected";
  $("#sync-status").classList.toggle("connected", state.sync.configured);
  $("#sync-side-status").textContent = state.sync.configured ? `${pending ? `${pending} pending · ` : ""}Google sync ready · v0.2.0` : "Private by default · v0.2.0";
}

function pendingDetected() {
  const names = new Set(state.bundle.catalog.map((entry) => entry.name.trim().toLowerCase()));
  return state.detected.filter((entry) => !names.has(entry.name.trim().toLowerCase()));
}
function visibleDetected() { const query = state.detectedQuery.trim().toLowerCase(); return pendingDetected().filter((entry) => [entry.name, entry.publisher, entry.version, entry.installed_path].join(" ").toLowerCase().includes(query)); }

function renderDetected() {
  const pending = pendingDetected(); const visible = visibleDetected(); const list = $("#detected-list"); list.replaceChildren();
  $("#detected-toolbar").classList.toggle("hidden", state.detected.length === 0);
  $("#detect-loading").classList.toggle("hidden", state.detected.length > 0);
  if (!pending.length && state.detected.length) {
    list.innerHTML = `<div class="empty"><div class="empty-icon">✓</div><h2>Everything found is already cataloged</h2><p>No review items remain on this device.</p></div>`;
  } else if (!visible.length && pending.length) {
    list.innerHTML = `<div class="empty"><h2>No detected software matches</h2><p>Clear the filter to see all pending scan results.</p></div>`;
  }
  for (const item of visible) {
    const key = detectedKey(item); const node = document.createElement("div"); node.className = "review-item detected-row";
    node.innerHTML = `<input class="review-check" type="checkbox"><div class="app-avatar"></div><div class="review-copy"><strong></strong><small></small></div><span class="pill success"></span>`;
    const checkbox = node.querySelector(".review-check"); checkbox.checked = state.detectedSelected.has(key); checkbox.setAttribute("aria-label", `Select ${item.name}`);
    node.querySelector(".app-avatar").textContent = initials(item.name); node.querySelector("strong").textContent = item.name;
    node.querySelector("small").textContent = [item.publisher, item.version, item.installed_path].filter(Boolean).join(" · "); node.querySelector(".pill").textContent = item.source_type;
    checkbox.addEventListener("change", (event) => { event.target.checked ? state.detectedSelected.add(key) : state.detectedSelected.delete(key); updateDetectedSelection(); });
    list.append(node);
  }
  updateDetectedSelection(visible);
}

function updateDetectedSelection(visible = visibleDetected()) {
  const pendingKeys = new Set(pendingDetected().map(detectedKey));
  state.detectedSelected = new Set([...state.detectedSelected].filter((key) => pendingKeys.has(key)));
  const count = state.detectedSelected.size; $("#add-selected-detected").disabled = count === 0; $("#add-all-detected").disabled = pendingKeys.size === 0;
  const summary = $("#detected-selection-summary"); summary.classList.toggle("hidden", count === 0);
  summary.textContent = count ? `${count} detected application${count === 1 ? "" : "s"} selected for this catalog.` : "";
  const visibleKeys = visible.map(detectedKey); $("#select-all-detected").disabled = visibleKeys.length === 0;
}

async function addDetected(items) {
  if (!items.length) return;
  const result = await call("add_detected_software_bulk", { detected: items }); state.bundle = result.bundle;
  items.forEach((item) => state.detectedSelected.delete(detectedKey(item))); renderDetected(); renderCatalog(); await loadSyncState();
  const uploadHint = state.sync.configured ? " Use Sync with Sheet to send them to your other devices." : " Set up Google synchronization when you want to share them.";
  toast(`${result.catalog_added} catalog item${result.catalog_added === 1 ? "" : "s"} added and ${result.installations_recorded} installation record${result.installations_recorded === 1 ? "" : "s"} saved.${uploadHint}`);
}

async function runDownloads() { const ids = selectedIds(); if (!ids.length) return; showProgress("Preparing downloads…", 0); const results = await call("download_selected", { ids }); hideProgress(); await refresh(); const failed = results.filter((result) => result.error || !result.checksum_ok); toast(failed.length ? `${results.length - failed.length} downloaded; ${failed.length} need attention` : `${results.length} download(s) completed`, failed.length ? "error" : "success"); }
async function runInstalls() { const ids = selectedIds(); if (!ids.length || !confirm(`Install ${ids.length} selected application(s) sequentially? System permission prompts may appear.`)) return; showProgress("Installing sequentially…", 0); const results = await call("install_selected", { ids }); hideProgress(); await refresh(); const failed = results.filter((result) => result.error || !result.outcome?.success); toast(failed.length ? `${results.length - failed.length} completed; ${failed.length} failed` : `${results.length} installation(s) completed`, failed.length ? "error" : "success"); }
function showProgress(label, value) { $("#bulk-status").classList.remove("hidden"); $("#progress-label").textContent = label; $("#progress-bar").value = value; }
function hideProgress() { $("#bulk-status").classList.add("hidden"); }

async function showSuggestions() {
  const suggestions = await call("find_installer_files"); const list = $("#suggestion-list"); list.replaceChildren();
  if (!suggestions.length) list.innerHTML = `<div class="empty"><h2>No confident matches</h2><p>Download installer files into your configured folder, or enable subfolder search.</p></div>`;
  for (const suggestion of suggestions) {
    const entry = state.bundle.catalog.find((candidate) => candidate.id === suggestion.software_id); const node = document.createElement("div"); node.className = "review-item";
    node.innerHTML = `<div class="app-avatar"></div><div class="review-copy"><strong></strong><small></small></div><span class="pill success"></span><button class="button accent">Confirm</button>`;
    node.querySelector(".app-avatar").textContent = initials(entry?.name); node.querySelector("strong").textContent = entry?.name || suggestion.software_id; node.querySelector("small").textContent = suggestion.path; node.querySelector(".pill").textContent = `${Math.round(suggestion.score * 100)}%`;
    node.querySelector("button").addEventListener("click", async () => { state.bundle = await call("confirm_installer_file", { softwareId: suggestion.software_id, path: suggestion.path }); node.remove(); renderCatalog(); toast("Installer linked"); }); list.append(node);
  }
  $("#suggestion-dialog").showModal();
}

function showHelp(button) { $("#help-title").textContent = button.dataset.helpTitle; $("#help-copy").textContent = button.dataset.help; $("#help-dialog").showModal(); }

function showSyncResult(report) {
  const labels = [
    ["Downloaded catalog", report.pulled_catalog], ["Downloaded installations", report.pulled_installations],
    ["Uploaded catalog", report.pushed_catalog], ["Uploaded installations", report.pushed_installations],
    ["Applied removals", (report.removed_catalog || 0) + (report.removed_installations || 0)], ["Uploaded removals", report.pushed_deletions],
  ];
  const grid = $("#sync-result-grid"); grid.replaceChildren();
  for (const [label, value] of labels) { const item = document.createElement("article"); const strong = document.createElement("strong"); strong.textContent = value || 0; const small = document.createElement("small"); small.textContent = label; item.append(strong, small); grid.append(item); }
  $("#sync-result-dialog").showModal();
}

async function performSync(pushAll = false) {
  if (!state.sync.configured) { await openScriptSetup(); return; }
  $("#sync-button").disabled = true; $("#sync-label").textContent = pushAll ? "Pushing library…" : "Synchronizing…";
  try {
    const result = await call(pushAll ? "push_entire_library" : "sync_now"); state.bundle = result.bundle; renderCatalog(); await loadSyncState(); showSyncResult(result.report);
  } finally { $("#sync-button").disabled = false; updateSyncUi(); }
}

async function openScriptSetup() {
  const setup = await call("prepare_apps_script"); $("#apps-script-code").value = setup.script; $("#script-source-link").href = setup.source_url;
  $("#wizard-script-url").value = state.settings.apps_script_url || ""; $("#wizard-connection-result").textContent = state.sync.configured ? "A connection is already saved. You may test it again or push the full library." : "Not connected yet.";
  $("#push-library").disabled = !state.sync.configured; $("#script-setup-dialog").showModal();
}

async function copyScript() {
  const code = $("#apps-script-code").value;
  try { await navigator.clipboard.writeText(code); }
  catch { $("#apps-script-code").select(); document.execCommand("copy"); }
  $("#copy-script").textContent = "Copied — paste into Apps Script"; setTimeout(() => { $("#copy-script").textContent = "Copy complete script"; }, 3000); toast("Complete version-matched script copied");
}

function settingsPayload(form, overrides = {}) {
  const raw = formObject(form);
  return { ...state.settings, ...raw, ...overrides, recursive_installer_search: form.elements.recursive_installer_search.checked, has_shared_secret: state.settings.has_shared_secret };
}

async function saveAndTestScript() {
  const url = $("#wizard-script-url").value.trim(); const resultNode = $("#wizard-connection-result");
  if (!/^https:\/\/script\.google\.com\/macros\/s\/.+\/exec(?:\?.*)?$/.test(url)) { resultNode.textContent = "Paste the deployed Web App URL ending in /exec."; resultNode.classList.add("error-text"); return; }
  resultNode.classList.remove("error-text"); resultNode.textContent = "Saving and contacting Google…";
  state.settings = await call("save_settings", { settings: settingsPayload($("#settings-form"), { apps_script_url: url }), sharedSecret: null });
  $("#settings-form").elements.apps_script_url.value = url;
  const ping = await call("test_apps_script_connection"); await loadSyncState();
  resultNode.textContent = `Connected to ${ping.sheet_name || "your Google Sheet"}. Found ${ping.catalog_count} catalog and ${ping.installations_count} installation rows.`;
  $("#push-library").disabled = false; toast("Google synchronization is connected");
}

async function removeSelected() {
  const ids = selectedIds(); if (!ids.length) return;
  if (!confirm(`Remove ${ids.length} selected item${ids.length === 1 ? "" : "s"} from the Installor catalog? This does not uninstall software or delete files.`)) return;
  const removed = await call("delete_catalog_bulk", { ids }); state.selected.clear(); await refresh(); await loadSyncState();
  toast(`${removed} catalog item${removed === 1 ? "" : "s"} removed${state.sync.configured ? "; synchronize to update other devices" : " locally"}`);
}

function closeAll(selector) { $$(selector).forEach((button) => button.addEventListener("click", () => button.closest("dialog").close())); }

function bind() {
  $$(".nav-item").forEach((button) => button.addEventListener("click", () => setView(button.dataset.view)));
  $$(".info-button[data-help]").forEach((button) => button.addEventListener("click", () => showHelp(button)));
  $("#add-button").addEventListener("click", () => openSoftware()); $("[data-action='add']").addEventListener("click", () => openSoftware()); $("[data-action='settings']").addEventListener("click", () => setView("settings"));
  closeAll("[data-close], [data-close-suggestions], [data-close-script], [data-close-help], [data-close-sync], [data-close-machine]");
  $("#software-form").elements.source_type.addEventListener("change", toggleDownloadField);
  $("#software-form").addEventListener("submit", async (event) => { event.preventDefault(); const entry = formObject(event.currentTarget); const errors = validateCatalog(entry); if (errors.length) return toast(errors.join(". "), "error"); await call("save_catalog", { entry }); $("#software-dialog").close(); await refresh(); await loadSyncState(); toast("Software saved"); });
  $("#search").addEventListener("input", (event) => { state.query = event.target.value; renderCatalog(); }); $("#status-filter").addEventListener("change", (event) => { state.statusFilter = event.target.value; renderCatalog(); });
  $("#select-all").addEventListener("click", () => { visibleCatalog().forEach((entry) => state.selected.add(entry.id)); renderCatalog(); }); $("#select-none").addEventListener("click", () => { state.selected.clear(); renderCatalog(); });
  $("#select-all-checkbox").addEventListener("change", (event) => { visibleCatalog().forEach((entry) => event.target.checked ? state.selected.add(entry.id) : state.selected.delete(entry.id)); renderCatalog(); });
  $("#remove-selected").addEventListener("click", removeSelected); $("#download-selected").addEventListener("click", runDownloads); $("#install-selected").addEventListener("click", runInstalls); $("#find-installers").addEventListener("click", showSuggestions);
  $("#detect-button").addEventListener("click", async () => { $("#detect-loading").classList.remove("hidden"); $("#detect-loading h2").textContent = "Scanning this device…"; state.detected = await call("detect_installed_software"); state.detectedSelected.clear(); renderDetected(); toast(`Found ${state.detected.length} reviewable software entries`); });
  $("#detected-search").addEventListener("input", (event) => { state.detectedQuery = event.target.value; renderDetected(); });
  $("#select-all-detected").addEventListener("click", () => { visibleDetected().forEach((item) => state.detectedSelected.add(detectedKey(item))); renderDetected(); }); $("#clear-detected").addEventListener("click", () => { state.detectedSelected.clear(); renderDetected(); });
  $("#add-selected-detected").addEventListener("click", () => addDetected(pendingDetected().filter((item) => state.detectedSelected.has(detectedKey(item))))); $("#add-all-detected").addEventListener("click", () => addDetected(pendingDetected()));
  $("#settings-form").addEventListener("submit", async (event) => { event.preventDefault(); const raw = formObject(event.currentTarget); state.settings = await call("save_settings", { settings: settingsPayload(event.currentTarget), sharedSecret: raw.shared_secret || null }); event.currentTarget.elements.shared_secret.value = ""; await loadSettings(); await loadSyncState(); renderCatalog(); toast("Settings saved"); });
  $("#import-sheet").addEventListener("click", async () => { const url = $("#settings-form").elements.public_sheet_url.value.trim(); if (!url) return toast("Paste a Google Sheet share URL first", "error"); state.bundle = await call("import_public_sheet", { url }); await loadSettings(); renderCatalog(); setView("catalog"); toast(`Imported ${state.bundle.catalog.length} catalog entries`); });
  $("#test-connection").addEventListener("click", async () => { const result = await call("test_apps_script_connection"); await loadSyncState(); toast(`Connected to ${result.sheet_name || "your Sheet"} (${result.catalog_count} catalog rows)`); });
  $("#sync-button").addEventListener("click", () => performSync(false)); $("#open-script-setup").addEventListener("click", openScriptSetup); $("#copy-script").addEventListener("click", copyScript); $("#save-test-script").addEventListener("click", saveAndTestScript);
  $("#push-library").addEventListener("click", async () => { await performSync(true); $("#script-setup-dialog").close(); });
  window.__TAURI__.event.listen("download-progress", ({ payload }) => { const percentage = payload.total ? Math.round(payload.downloaded / payload.total * 100) : 0; showProgress("Downloading…", percentage); $("#progress-value").textContent = payload.total ? `${percentage}%` : `${Math.round(payload.downloaded / 1024)} KB`; });
  window.__TAURI__.event.listen("sync-complete", async () => { await refresh(); await loadSyncState(); toast("Startup synchronization complete"); });
}

async function start() { bind(); await loadSettings(); await refresh(); await loadSyncState(); updateCatalogSelection(); }
start().catch((error) => { $("#local-status").textContent = "Startup error"; console.error(error); });
