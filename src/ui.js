export function catalogStatus(entry, installations, machineLabel) {
  const matching = installations.filter((i) => i.software_id === entry.id && i.installed);
  if (matching.some((i) => i.machine_label === machineLabel)) return { key: "here", label: "Installed here" };
  if (matching.length) return { key: "elsewhere", label: "Installed elsewhere" };
  return { key: "missing", label: "Not installed" };
}

export function validateCatalog(entry) {
  const errors = [];
  if (!entry.name?.trim()) errors.push("Name is required");
  if (entry.checksum_sha256 && !/^[a-f\d]{64}$/i.test(entry.checksum_sha256)) errors.push("SHA-256 must contain 64 hexadecimal characters");
  if (entry.source_type !== "custom" && !entry.source_id?.trim()) errors.push("A package ID is required for package-manager sources");
  return errors;
}

export function matchesSearch(entry, query) {
  const haystack = [entry.name, entry.publisher, entry.category, entry.tags, entry.source_id].join(" ").toLowerCase();
  return haystack.includes(query.trim().toLowerCase());
}

export function selectAllIds(entries) { return new Set(entries.map((entry) => entry.id)); }
