export function catalogStatus(entry, installations, machineLabel) {
  const matching = installations.filter((i) => i.software_id === entry.id && i.installed);
  const machineLabels = [...new Set(matching.map((i) => i.machine_label).filter(Boolean))].sort();
  const here = matching.some((i) => i.machine_label === machineLabel);
  if (here) return { key: "here", label: `Installed on ${machineLabel} (this device)`, machineLabels, installations: matching };
  if (matching.length) return { key: "elsewhere", label: `Installed on ${machineLabels.join(", ") || "another machine"}`, machineLabels, installations: matching };
  return { key: "missing", label: "Not installed here", machineLabels: [], installations: [] };
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

export function matchesStatus(entry, installations, machineLabel, filter) {
  if (filter.startsWith("device:")) {
    const deviceName = filter.slice("device:".length);
    return installations.some((row) => row.software_id === entry.id && row.installed && row.machine_label === deviceName);
  }
  return filter === "all" || catalogStatus(entry, installations, machineLabel).key === filter;
}

export function detectedKey(entry) {
  return [entry.name, entry.publisher, entry.version, entry.installed_path, entry.source_id]
    .map((value) => String(value || "").trim().toLowerCase())
    .join("|");
}

export function selectAllIds(entries) { return new Set(entries.map((entry) => entry.id)); }
