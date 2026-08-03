// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { catalogStatus, matchesSearch, selectAllIds, validateCatalog } from "./ui.js";

describe("catalog UI", () => {
  const entry = { id: "a", name: "OBS Studio", publisher: "OBS", category: "Video", tags: "recording", source_id: "OBSProject.OBSStudio" };
  it("selects and clears catalog rows", () => { const selected = selectAllIds([entry, { id: "b" }]); expect([...selected]).toEqual(["a", "b"]); selected.clear(); expect(selected.size).toBe(0); });
  it("validates forms", () => { expect(validateCatalog({ name: "", source_type: "winget", source_id: "" })).toHaveLength(2); expect(validateCatalog({ name: "OBS", source_type: "custom", checksum_sha256: "" })).toEqual([]); });
  it("filters rendered data", () => { expect(matchesSearch(entry, "recording")).toBe(true); expect(matchesSearch(entry, "browser")).toBe(false); });
  it("calculates installation status", () => { expect(catalogStatus(entry, [{ software_id: "a", installed: true, machine_label: "PC" }], "PC").key).toBe("here"); expect(catalogStatus(entry, [{ software_id: "a", installed: true, machine_label: "Mac" }], "PC").key).toBe("elsewhere"); });
});
