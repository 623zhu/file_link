import { describe, expect, it } from "vitest";

import {
  applySelection,
  selectionModeFromEvent,
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
