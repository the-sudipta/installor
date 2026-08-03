import { catalogStatus, matchesSearch, selectAllIds, validateCatalog } from "./ui.js";

const invoke = (...args) => window.__TAURI__.core.invoke(...args);
const state = { bundle: { catalog: [], installations: [] }, settings: {}, selected: new Set(), detected: [], query: "" };
const $ = (selector) => document.querySelector(selector);
const $$ = (selector) => [...document.querySelectorAll(selector)];

function toast(message, type = "success") { const node = document.createElement("div"); node.className = `toast ${type}`; node.textContent = message; $("#toast-region").append(node); setTimeout(() => node.remove(), 4500); }
async function call(command, args = {}) { try { return await invoke(command, args); } catch (error) { toast(String(error), "error"); throw error; } }
function initials(name = "?") { return name.split(/\s+/).slice(0, 2).map((p) => p[0]).join("").toUpperCase(); }
function selectedIds() { return [...state.selected]; }

function setView(view) { $$(".view").forEach((v) => v.classList.toggle("active", v.id === `${view}-view`)); $$(".nav-item").forEach((b) => b.classList.toggle("active", b.dataset.view === view)); const titles = { catalog: "Your catalog", detected: "Detected software", settings: "Settings & sync" }; $("#view-title").textContent = titles[view]; $("#add-button").classList.toggle("hidden", view !== "catalog"); }
function currentInstall(entry) { return state.bundle.installations.find((i) => i.software_id === entry.id && i.machine_label === state.settings.machine_label); }

function renderCatalog() {
  const entries = state.bundle.catalog.filter((entry) => matchesSearch(entry, state.query));
  const body = $("#catalog-body"); body.replaceChildren();
  for (const entry of entries) {
    const status = catalogStatus(entry, state.bundle.installations, state.settings.machine_label);
    const row = document.createElement("tr"); row.dataset.id = entry.id;
    const installer = currentInstall(entry)?.installer_file_path;
    row.innerHTML = `<td><input class="row-check" type="checkbox" aria-label="Select ${entry.name}" ${state.selected.has(entry.id) ? "checked" : ""}></td><td><strong></strong><small></small></td><td></td><td></td><td><span class="status ${status.key}">${status.label}</span></td><td><strong>${entry.source_type}</strong><small></small></td><td class="align-right"><div class="row-actions"><button class="icon-button edit" title="Edit">✎</button><button class="icon-button danger delete" title="Delete">×</button></div></td>`;
    const cells = row.children; cells[1].querySelector("strong").textContent = entry.name; cells[1].querySelector("small").textContent = entry.tags || installer || ""; cells[2].textContent = entry.publisher || "—"; cells[3].textContent = entry.category || "—"; cells[5].querySelector("small").textContent = entry.source_id || entry.installer_type;
    row.querySelector(".row-check").addEventListener("change", (event) => { event.target.checked ? state.selected.add(entry.id) : state.selected.delete(entry.id); updateSelection(); });
    row.querySelector(".edit").addEventListener("click", () => openSoftware(entry));
    row.querySelector(".delete").addEventListener("click", async () => { if (confirm(`Remove ${entry.name} from your catalog?`)) { await call("delete_catalog", { id: entry.id }); await refresh(); } });
    body.append(row);
  }
  $("#empty-catalog").classList.toggle("hidden", state.bundle.catalog.length !== 0);
  $("#catalog-count").textContent = state.bundle.catalog.length;
  $("#installed-count").textContent = state.bundle.catalog.filter((e) => catalogStatus(e, state.bundle.installations, state.settings.machine_label).key === "here").length;
  $("#ready-count").textContent = state.bundle.catalog.filter((e) => e.source_type !== "custom" ? e.source_id : e.download_url || currentInstall(e)?.installer_file_path).length;
  updateSelection();
}
function updateSelection() { const n = state.selected.size; $("#download-selected").disabled = !n; $("#install-selected").disabled = !n; $("#select-all-checkbox").checked = n > 0 && n === state.bundle.catalog.length; }
async function refresh() { state.bundle = await call("get_catalog"); renderCatalog(); }

function openSoftware(entry = null) { const form = $("#software-form"); form.reset(); $("#dialog-title").textContent = entry ? "Edit software" : "Add software"; if (entry) Object.entries(entry).forEach(([key, value]) => { if (form.elements[key]) form.elements[key].value = value ?? ""; }); toggleDownloadField(); $("#software-dialog").showModal(); }
function formObject(form) { return Object.fromEntries(new FormData(form).entries()); }
function toggleDownloadField() { $("#download-url-field").classList.toggle("hidden", $("#software-form").elements.source_type.value !== "custom"); }

async function loadSettings() { state.settings = await call("get_settings"); const form = $("#settings-form"); for (const [key, value] of Object.entries(state.settings)) { if (!form.elements[key]) continue; if (form.elements[key].type === "checkbox") form.elements[key].checked = value; else form.elements[key].value = value ?? ""; } $("#secret-status").textContent = state.settings.has_shared_secret ? "A secret is already stored; leave blank to keep it." : "No shared secret stored."; }

function renderDetected() { const list = $("#detected-list"); list.replaceChildren(); const catalogNames = new Set(state.bundle.catalog.map((e) => e.name.toLowerCase())); const pending = state.detected.filter((d) => !catalogNames.has(d.name.toLowerCase())); $("#detect-loading").classList.toggle("hidden", state.detected.length > 0); if (!pending.length && state.detected.length) { list.innerHTML = `<div class="empty"><h2>Everything found is already cataloged</h2><p>No review items remain.</p></div>`; return; } for (const item of pending) { const node = document.createElement("div"); node.className = "review-item"; node.innerHTML = `<div class="app-avatar"></div><div class="review-copy"><strong></strong><small></small></div><span class="pill success"></span><button class="button accent">Add to catalog</button>`; node.querySelector(".app-avatar").textContent = initials(item.name); node.querySelector("strong").textContent = item.name; node.querySelector("small").textContent = [item.publisher, item.version, item.installed_path].filter(Boolean).join(" · "); node.querySelector(".pill").textContent = item.source_type; node.querySelector("button").addEventListener("click", async () => { state.bundle = await call("add_detected_software", { detected: item }); renderDetected(); renderCatalog(); toast(`${item.name} added`); }); list.append(node); } }

async function runDownloads() { const ids = selectedIds(); if (!ids.length) return; showProgress("Preparing downloads…", 0); const results = await call("download_selected", { ids }); hideProgress(); await refresh(); const failed = results.filter((r) => r.error || !r.checksum_ok); toast(failed.length ? `${results.length - failed.length} downloaded; ${failed.length} need attention` : `${results.length} download(s) completed`, failed.length ? "error" : "success"); }
async function runInstalls() { const ids = selectedIds(); if (!ids.length || !confirm(`Install ${ids.length} selected application(s) sequentially? System permission prompts may appear.`)) return; showProgress("Installing sequentially…", 0); const results = await call("install_selected", { ids }); hideProgress(); await refresh(); const failed = results.filter((r) => r.error || !r.outcome?.success); toast(failed.length ? `${results.length - failed.length} completed; ${failed.length} failed` : `${results.length} installation(s) completed`, failed.length ? "error" : "success"); }
function showProgress(label, value) { $("#bulk-status").classList.remove("hidden"); $("#progress-label").textContent = label; $("#progress-bar").value = value; }
function hideProgress() { $("#bulk-status").classList.add("hidden"); }

async function showSuggestions() { const suggestions = await call("find_installer_files"); const list = $("#suggestion-list"); list.replaceChildren(); if (!suggestions.length) list.innerHTML = `<div class="empty"><h2>No confident matches</h2><p>Download installer files into your configured folder, or enable subfolder search.</p></div>`; for (const suggestion of suggestions) { const entry = state.bundle.catalog.find((e) => e.id === suggestion.software_id); const node = document.createElement("div"); node.className = "review-item"; node.innerHTML = `<div class="app-avatar">${initials(entry?.name)}</div><div class="review-copy"><strong></strong><small></small></div><span class="pill success"></span><button class="button accent">Confirm</button>`; node.querySelector("strong").textContent = entry?.name || suggestion.software_id; node.querySelector("small").textContent = suggestion.path; node.querySelector(".pill").textContent = `${Math.round(suggestion.score * 100)}%`; node.querySelector("button").addEventListener("click", async () => { state.bundle = await call("confirm_installer_file", { softwareId: suggestion.software_id, path: suggestion.path }); node.remove(); renderCatalog(); toast("Installer linked"); }); list.append(node); } $("#suggestion-dialog").showModal(); }

function bind() {
  $$(".nav-item").forEach((b) => b.addEventListener("click", () => setView(b.dataset.view))); $("#add-button").addEventListener("click", () => openSoftware()); $("[data-action='add']").addEventListener("click", () => openSoftware()); $("[data-action='settings']").addEventListener("click", () => setView("settings"));
  $$("[data-close]").forEach((b) => b.addEventListener("click", () => $("#software-dialog").close())); $("[data-close-suggestions]").addEventListener("click", () => $("#suggestion-dialog").close()); $("#software-form").elements.source_type.addEventListener("change", toggleDownloadField);
  $("#software-form").addEventListener("submit", async (event) => { event.preventDefault(); const entry = formObject(event.currentTarget); const errors = validateCatalog(entry); if (errors.length) return toast(errors.join(". "), "error"); await call("save_catalog", { entry }); $("#software-dialog").close(); await refresh(); toast("Software saved"); });
  $("#search").addEventListener("input", (e) => { state.query = e.target.value; renderCatalog(); }); $("#select-all").addEventListener("click", () => { state.selected = selectAllIds(state.bundle.catalog); renderCatalog(); }); $("#select-none").addEventListener("click", () => { state.selected.clear(); renderCatalog(); }); $("#select-all-checkbox").addEventListener("change", (e) => { state.selected = e.target.checked ? selectAllIds(state.bundle.catalog) : new Set(); renderCatalog(); });
  $("#download-selected").addEventListener("click", runDownloads); $("#install-selected").addEventListener("click", runInstalls); $("#find-installers").addEventListener("click", showSuggestions);
  $("#detect-button").addEventListener("click", async () => { $("#detect-loading").classList.remove("hidden"); $("#detect-loading h2").textContent = "Scanning…"; state.detected = await call("detect_installed_software"); renderDetected(); toast(`Found ${state.detected.length} software entries`); });
  $("#settings-form").addEventListener("submit", async (event) => { event.preventDefault(); const raw = formObject(event.currentTarget); const settings = { ...state.settings, ...raw, recursive_installer_search: event.currentTarget.elements.recursive_installer_search.checked, has_shared_secret: state.settings.has_shared_secret }; state.settings = await call("save_settings", { settings, sharedSecret: raw.shared_secret || null }); event.currentTarget.elements.shared_secret.value = ""; await loadSettings(); renderCatalog(); toast("Settings saved"); });
  $("#import-sheet").addEventListener("click", async () => { const url = $("#settings-form").elements.public_sheet_url.value.trim(); if (!url) return toast("Paste a Google Sheet share URL first", "error"); state.bundle = await call("import_public_sheet", { url }); await loadSettings(); renderCatalog(); setView("catalog"); toast(`Imported ${state.bundle.catalog.length} catalog entries`); });
  $("#test-connection").addEventListener("click", async () => { const result = await call("test_apps_script_connection"); toast(`Connected to ${result.sheet_name} (${result.catalog_count} catalog rows)`); });
  $("#sync-button").addEventListener("click", async () => { state.bundle = await call("sync_now"); renderCatalog(); toast("Sync complete"); });
  window.__TAURI__.event.listen("download-progress", ({ payload }) => { const pct = payload.total ? Math.round(payload.downloaded / payload.total * 100) : 0; showProgress("Downloading…", pct); $("#progress-value").textContent = payload.total ? `${pct}%` : `${Math.round(payload.downloaded / 1024)} KB`; });
  window.__TAURI__.event.listen("sync-complete", async () => { await refresh(); toast("Startup sync complete"); });
}

async function start() { bind(); await loadSettings(); await refresh(); updateSelection(); }
start().catch((error) => { $("#local-status").textContent = "Startup error"; console.error(error); });
