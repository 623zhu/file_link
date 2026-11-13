import { describe, expect, it } from "vitest";

import {
  applySelection,
  browseTabFromView,
  nextCursorIndex,
  parseStoredView,
  resolveView,
  selectionModeFromEvent,
  viewForBrowseTab,
  type SelectionState,
} from "./store";

const ids = ["a", "b", "c", "d"];

function selection(
  selected: string[],
  anchorId: string | null = null,
): SelectionState {
  return { selectedIds: new Set(selected), anchorId };
}

function selectedOf(state: SelectionState): string[] {
  return [...state.selectedIds];
}

describe("selectionModeFromEvent", () => {
  it("uses replace, toggle, range, and add-range from modifiers", () => {
    expect(
      selectionModeFromEvent({
        shiftKey: false,
        ctrlKey: false,
        metaKey: false,
      }),
    ).toBe("replace");
    expect(
      selectionModeFromEvent({
        shiftKey: false,
        ctrlKey: true,
        metaKey: false,
      }),
    ).toBe("toggle");
    expect(
      selectionModeFromEvent({
        shiftKey: false,
        ctrlKey: false,
        metaKey: true,
      }),
    ).toBe("toggle");
    expect(
      selectionModeFromEvent({
        shiftKey: true,
        ctrlKey: false,
        metaKey: false,
      }),
    ).toBe("range");
    expect(
      selectionModeFromEvent({ shiftKey: true, ctrlKey: true, metaKey: false }),
    ).toBe("add-range");
  });
});

describe("applySelection", () => {
  it("replaces the selection and moves the anchor", () => {
    const next = applySelection(
      selection(["a", "c"], "a"),
      ids,
      "b",
      "replace",
    );
    expect(selectedOf(next)).toEqual(["b"]);
    expect(next.anchorId).toBe("b");
  });

  it("toggles a row in or out without dropping the rest", () => {
    const added = applySelection(selection(["a"], "a"), ids, "c", "toggle");
    expect(selectedOf(added).sort()).toEqual(["a", "c"]);
    expect(added.anchorId).toBe("c");

    const removed = applySelection(added, ids, "a", "toggle");
    expect(selectedOf(removed)).toEqual(["c"]);
    expect(removed.anchorId).toBe("a");
  });

  it("selects the inclusive range from the anchor, including reverse", () => {
    const down = applySelection(selection(["b"], "b"), ids, "d", "range");
    expect(selectedOf(down)).toEqual(["b", "c", "d"]);
    expect(down.anchorId).toBe("b");

    const up = applySelection(selection(["b"], "b"), ids, "a", "range");
    expect(selectedOf(up)).toEqual(["a", "b"]);
    expect(up.anchorId).toBe("b");
  });

  it("starts a range from the target when there is no usable anchor", () => {
    const next = applySelection(selection([]), ids, "c", "range");
    expect(selectedOf(next)).toEqual(["c"]);
    expect(next.anchorId).toBe("c");
  });

  it("adds a range onto the current selection", () => {
    const next = applySelection(selection(["a"], "a"), ids, "c", "add-range");
    expect(selectedOf(next).sort()).toEqual(["a", "b", "c"]);
    expect(next.anchorId).toBe("a");
  });

  it("ignores a target that is not in the visible list", () => {
    const current = selection(["a"], "a");
    const next = applySelection(current, ids, "missing", "toggle");
    expect(selectedOf(next)).toEqual(["a"]);
    expect(next.anchorId).toBe("a");
  });
});

describe("nextCursorIndex", () => {
  it("moves by one and stops at both ends", () => {
    expect(nextCursorIndex(4, 1, 1)).toBe(2);
    expect(nextCursorIndex(4, 1, -1)).toBe(0);
    expect(nextCursorIndex(4, 0, -1)).toBe(0);
    expect(nextCursorIndex(4, 3, 1)).toBe(3);
  });

  it("starts from the first or last row when there is no cursor", () => {
    expect(nextCursorIndex(4, -1, 1)).toBe(0);
    expect(nextCursorIndex(4, -1, -1)).toBe(3);
  });

  it("returns -1 for an empty list", () => {
    expect(nextCursorIndex(0, -1, 1)).toBe(-1);
    expect(nextCursorIndex(0, 2, -1)).toBe(-1);
  });
});

describe("parseStoredView", () => {
  it("restores binding and extension views", () => {
    expect(parseStoredView('{"type":"binding","id":"b2"}')).toEqual({
      type: "binding",
      id: "b2",
    });
    expect(parseStoredView('{"type":"extension","ext":".md"}')).toEqual({
      type: "extension",
      ext: ".md",
    });
  });

  it("falls back to none for missing, broken, or unknown content", () => {
    expect(parseStoredView(null)).toEqual({ type: "none" });
    expect(parseStoredView("{not json")).toEqual({ type: "none" });
    expect(parseStoredView('"binding"')).toEqual({ type: "none" });
    expect(parseStoredView('{"type":"binding","id":3}')).toEqual({
      type: "none",
    });
    expect(parseStoredView('{"type":"all"}')).toEqual({ type: "none" });
  });

  it("is validated against current bindings by resolveView", () => {
    const bindings = [
      {
        id: "b1",
        path: "D:\\a",
        key: "d:\\a",
        recursive: false,
        autoWatch: false,
        extensions: [".html"],
        ruleId: null,
        createdAt: "2026-11-04T15:00:00+08:00",
        unavailable: false,
      },
    ];
    const stored = parseStoredView('{"type":"binding","id":"removed"}');
    expect(resolveView(stored, bindings)).toEqual({ type: "binding", id: "b1" });
  });
});

describe("browseTabFromView", () => {
  it("maps extension view to the type tab", () => {
    expect(browseTabFromView({ type: "extension", ext: ".html" })).toBe("type");
  });

  it("maps binding and none to the folder tab", () => {
    expect(browseTabFromView({ type: "binding", id: "b1" })).toBe("folder");
    expect(browseTabFromView({ type: "none" })).toBe("folder");
  });
});

describe("viewForBrowseTab", () => {
  const bindings = [{ id: "b1" }, { id: "b2" }];
  const extensions = [".html", ".md"];

  it("restores the preferred binding when switching to folder", () => {
    expect(viewForBrowseTab("folder", bindings, extensions, "b2")).toEqual({
      type: "binding",
      id: "b2",
    });
  });

  it("falls back to the first binding when the preferred one is gone", () => {
    expect(viewForBrowseTab("folder", bindings, extensions, "gone")).toEqual({
      type: "binding",
      id: "b1",
    });
  });

  it("returns none when there are no bindings", () => {
    expect(viewForBrowseTab("folder", [], extensions)).toEqual({ type: "none" });
  });

  it("restores the preferred extension when switching to type", () => {
    expect(
      viewForBrowseTab("type", bindings, extensions, undefined, ".md"),
    ).toEqual({ type: "extension", ext: ".md" });
  });

  it("falls back to the first extension when the preferred one is gone", () => {
    expect(
      viewForBrowseTab("type", bindings, extensions, undefined, ".txt"),
    ).toEqual({ type: "extension", ext: ".html" });
  });

  it("returns none when there are no extensions", () => {
    expect(viewForBrowseTab("type", bindings, [])).toEqual({ type: "none" });
  });
});
