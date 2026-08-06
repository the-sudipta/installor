// Installor v0.2 - Google Apps Script Web App bridge.
// Deploy from Extensions > Apps Script inside your own Google Sheet.
const SHARED_SECRET = "REPLACE_WITH_YOUR_OWN_SECRET";
const CATALOG_SHEET = "Catalog";
const INSTALLATIONS_SHEET = "Installations";
const DELETIONS_SHEET = "Installor Deletions";
const CATALOG_COLUMNS = ["id","name","publisher","category","platforms","source_type","source_id","download_url","installer_type","silent_args","checksum_sha256","tags","notes","created_at","updated_at"];
const INSTALLATIONS_COLUMNS = ["id","software_id","machine_id","machine_label","installed","installed_version","installed_path","installer_file_path","last_verified_at","updated_at"];
const DELETION_COLUMNS = ["entity","id","deleted_at"];

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
      case "list": {
        const deletions = readRows_(ss, DELETIONS_SHEET, DELETION_COLUMNS);
        return jsonOut_({
          catalog: readRows_(ss, CATALOG_SHEET, CATALOG_COLUMNS),
          installations: readRows_(ss, INSTALLATIONS_SHEET, INSTALLATIONS_COLUMNS),
          deleted_catalog: deletions.filter(r => r.entity === "catalog").map(r => ({ id: r.id, deleted_at: r.deleted_at })),
          deleted_installations: deletions.filter(r => r.entity === "installation").map(r => ({ id: r.id, deleted_at: r.deleted_at }))
        });
      }
      case "upsert_catalog":
        return jsonOut_(upsertRow_(ss, CATALOG_SHEET, CATALOG_COLUMNS, params.row, "catalog"));
      case "delete_catalog":
        return jsonOut_(deleteRow_(ss, CATALOG_SHEET, params.id, "catalog"));
      case "upsert_installation":
        return jsonOut_(upsertRow_(ss, INSTALLATIONS_SHEET, INSTALLATIONS_COLUMNS, params.row, "installation"));
      case "delete_installation":
        return jsonOut_(deleteRow_(ss, INSTALLATIONS_SHEET, params.id, "installation"));
      default:
        return jsonOut_({ error: "unknown_action" });
    }
  } catch (err) {
    return jsonOut_({ error: String(err) });
  }
}

function ensureSheets_(ss) {
  ensureSheet_(ss, CATALOG_SHEET, CATALOG_COLUMNS);
  ensureSheet_(ss, INSTALLATIONS_SHEET, INSTALLATIONS_COLUMNS);
  ensureSheet_(ss, DELETIONS_SHEET, DELETION_COLUMNS);
}

function ensureSheet_(ss, name, columns) {
  let sh = ss.getSheetByName(name);
  if (!sh) sh = ss.insertSheet(name);
  if (sh.getLastRow() === 0) sh.getRange(1, 1, 1, columns.length).setValues([columns]);
}

function readRows_(ss, name, columns) {
  const sh = ss.getSheetByName(name), last = sh.getLastRow();
  if (last < 2) return [];
  return sh.getRange(2, 1, last - 1, columns.length).getValues()
    .filter(r => r[0] !== "")
    .map(r => { const obj = {}; columns.forEach((c, i) => obj[c] = r[i]); return obj; });
}

function findRowIndexById_(sh, id, idColumn) {
  const last = sh.getLastRow();
  if (last < 2) return -1;
  const ids = sh.getRange(2, idColumn, last - 1, 1).getValues();
  for (let i = 0; i < ids.length; i++) if (ids[i][0] === id) return i + 2;
  return -1;
}

function upsertRow_(ss, name, columns, row, entity) {
  const sh = ss.getSheetByName(name), lock = LockService.getScriptLock();
  lock.waitLock(10000);
  try {
    const values = columns.map(c => row[c] !== undefined ? row[c] : "");
    const idx = findRowIndexById_(sh, row.id, 1);
    if (idx === -1) sh.appendRow(values); else sh.getRange(idx, 1, 1, columns.length).setValues([values]);
    removeDeletion_(ss, entity, row.id);
    return { ok: true, id: row.id };
  } finally {
    lock.releaseLock();
  }
}

function deleteRow_(ss, name, id, entity) {
  const sh = ss.getSheetByName(name), lock = LockService.getScriptLock();
  lock.waitLock(10000);
  try {
    const idx = findRowIndexById_(sh, id, 1);
    if (idx !== -1) sh.deleteRow(idx);
    recordDeletion_(ss, entity, id);
    return { ok: true };
  } finally {
    lock.releaseLock();
  }
}

function recordDeletion_(ss, entity, id) {
  const sh = ss.getSheetByName(DELETIONS_SHEET);
  const rows = readRows_(ss, DELETIONS_SHEET, DELETION_COLUMNS);
  const existingIndex = rows.findIndex(r => r.entity === entity && r.id === id);
  const values = [entity, id, new Date().toISOString()];
  if (existingIndex === -1) sh.appendRow(values); else sh.getRange(existingIndex + 2, 1, 1, values.length).setValues([values]);
}

function removeDeletion_(ss, entity, id) {
  const sh = ss.getSheetByName(DELETIONS_SHEET);
  const rows = readRows_(ss, DELETIONS_SHEET, DELETION_COLUMNS);
  for (let i = rows.length - 1; i >= 0; i--) {
    if (rows[i].entity === entity && rows[i].id === id) sh.deleteRow(i + 2);
  }
}

function jsonOut_(obj) {
  return ContentService.createTextOutput(JSON.stringify(obj)).setMimeType(ContentService.MimeType.JSON);
}
