// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { catalogStatus, detectedKey, matchesSearch, matchesStatus, selectAllIds, validateCatalog } from "./ui.js";

describe("catalog UI", () => {
  const entry = { id: "a", name: "OBS Studio", publisher: "OBS", category: "Video", tags: "recording", source_id: "OBSProject.OBSStudio" };
  it("selects and clears catalog rows", () => { const selected = selectAllIds([entry, { id: "b" }]); expect([...selected]).toEqual(["a", "b"]); selected.clear(); expect(selected.size).toBe(0); });
  it("validates forms", () => { expect(validateCatalog({ name: "", source_type: "winget", source_id: "" })).toHaveLength(2); expect(validateCatalog({ name: "OBS", source_type: "custom", checksum_sha256: "" })).toEqual([]); });
  it("filters rendered data", () => { expect(matchesSearch(entry, "recording")).toBe(true); expect(matchesSearch(entry, "browser")).toBe(false); });
  it("calculates installation status with precise device names", () => { expect(catalogStatus(entry, [{ software_id: "a", installed: true, machine_label: "Office Desktop" }], "Office Desktop").label).toBe("Installed on Office Desktop (this device)"); expect(catalogStatus(entry, [{ software_id: "a", installed: true, machine_label: "Research Laptop" }], "Office Desktop").label).toBe("Installed on Research Laptop"); });
  it("filters by current-machine status", () => { expect(matchesStatus(entry, [{ software_id: "a", installed: true, machine_label: "Mac" }], "PC", "elsewhere")).toBe(true); expect(matchesStatus(entry, [], "PC", "here")).toBe(false); });
  it("filters by user-defined device category", () => { expect(matchesStatus(entry, [{ software_id: "a", installed: true, machine_label: "Research Laptop" }], "Office Desktop", "device:Research Laptop")).toBe(true); });
  it("creates stable detected-software keys", () => { expect(detectedKey({ name: " OBS ", publisher: "OBS", version: "1" })).toBe("obs|obs|1||"); });
});
