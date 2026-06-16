import { describe, expect, test } from "vitest";
import { MOCK_CONFLICT_DATA, MOCK_CONFLICT_DETAILS } from "./mockConflicts";

describe("mockConflicts", () => {
  test("provides at least one unresolved conflict", () => {
    expect(MOCK_CONFLICT_DATA.conflicts.length).toBeGreaterThan(0);
    expect(MOCK_CONFLICT_DATA.conflicts.every((c) => !c.resolved)).toBe(true);
  });

  test("provides a detail entry for every conflict", () => {
    for (const conflict of MOCK_CONFLICT_DATA.conflicts) {
      const detail = MOCK_CONFLICT_DETAILS[conflict.noteId];
      expect(detail).toBeDefined();
      expect(detail.noteId).toBe(conflict.noteId);
    }
  });

  test("detail note ids match the conflict list", () => {
    const conflictIds = new Set(MOCK_CONFLICT_DATA.conflicts.map((c) => c.noteId));
    const detailIds = Object.keys(MOCK_CONFLICT_DETAILS);
    expect(detailIds.every((id) => conflictIds.has(id))).toBe(true);
  });

  test("includes a nested category move conflict", () => {
    const nested = MOCK_CONFLICT_DATA.conflicts.find(
      (c) => c.localCategory.includes("/") && c.remoteCategory.includes("/"),
    );
    expect(nested).toBeDefined();
    expect(nested?.conflictType.categoryMoved).toBe(true);
    expect(nested?.localCategory).not.toBe(nested?.remoteCategory);
  });
});
