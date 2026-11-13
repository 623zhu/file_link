import { create } from "zustand";

import { getSnapshot } from "./api/commands";
import type { Binding, Link, Snapshot, View } from "./types";
import { listedExtensions } from "./utils/filter";

export type SelectMode = "replace" | "toggle" | "range" | "add-range";

export interface SelectionState {
  selectedIds: Set<string>;
  anchorId: string | null;
}

interface Store {
  snapshot: Snapshot | null;
  view: View;
  selectedIds: Set<string>;
  query: string;
  listFocusRequest: number;
  refresh: () => Promise<void>;
}

const VIEW_STORAGE_KEY = "file-link.view";

export function parseStoredView(raw: string | null): View {
  if (raw) {
    try {
      const value: unknown = JSON.parse(raw);
      if (typeof value === "object" && value !== null) {
        const item = value as Record<string, unknown>;
        if (item.type === "binding" && typeof item.id === "string") {
          return { type: "binding", id: item.id };
        }
        if (item.type === "extension" && typeof item.ext === "string") {
          return { type: "extension", ext: item.ext };
        }
      }
    } catch {
      // fall through to "none"
    }
  }
  return { type: "none" };
}

function loadStoredView(): View {
  try {
    return parseStoredView(
      globalThis.localStorage?.getItem(VIEW_STORAGE_KEY) ?? null,
    );
  } catch {
    return { type: "none" };
  }
}

function saveView(view: View) {
  if (view.type === "none") {
    return;
  }
  try {
    globalThis.localStorage?.setItem(VIEW_STORAGE_KEY, JSON.stringify(view));
  } catch {
    // storage unavailable; the view simply is not remembered
  }
}

export const useStore = create<Store>((set) => ({
  snapshot: null,
  view: loadStoredView(),
  selectedIds: new Set(),
  query: "",
  listFocusRequest: 0,
  refresh: async () => {
    const snapshot = await getSnapshot();
    set((state) => ({
      snapshot,
      view: resolveView(state.view, snapshot.bindings, snapshot.links),
    }));
  },
}));

useStore.subscribe((state, previous) => {
  if (state.view !== previous.view) {
    saveView(state.view);
  }
});

export type BrowseTab = "folder" | "type";

export function browseTabFromView(view: View): BrowseTab {
  return view.type === "extension" ? "type" : "folder";
}

export function viewForBrowseTab(
  tab: BrowseTab,
  bindings: readonly Pick<Binding, "id">[],
  extensions: readonly string[],
  preferredBindingId?: string,
  preferredExt?: string,
): View {
  if (tab === "folder") {
    const id =
      preferredBindingId &&
      bindings.some((item) => item.id === preferredBindingId)
        ? preferredBindingId
        : bindings[0]?.id;
    return id ? { type: "binding", id } : { type: "none" };
  }
  const ext =
    preferredExt && extensions.includes(preferredExt)
      ? preferredExt
      : extensions[0];
  return ext ? { type: "extension", ext } : { type: "none" };
}

export function resolveView(
  view: View,
  bindings: readonly Binding[],
  links: readonly Link[] = [],
): View {
  if (view.type === "binding" && bindings.some((item) => item.id === view.id)) {
    return view;
  }
  if (
    view.type === "extension" &&
    listedExtensions(links).includes(view.ext)
  ) {
    return view;
  }
  const first = bindings[0];
  if (first) {
    return { type: "binding", id: first.id };
  }
  const ext = listedExtensions(links)[0];
  return ext ? { type: "extension", ext } : { type: "none" };
}

export function selectionModeFromEvent(event: {
  shiftKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
}): SelectMode {
  const ctrl = event.ctrlKey || event.metaKey;
  if (event.shiftKey && ctrl) {
    return "add-range";
  }
  if (event.shiftKey) {
    return "range";
  }
  if (ctrl) {
    return "toggle";
  }
  return "replace";
}

export function nextCursorIndex(
  length: number,
  current: number,
  delta: number,
): number {
  if (length === 0) {
    return -1;
  }
  if (current < 0) {
    return delta > 0 ? 0 : length - 1;
  }
  return Math.min(Math.max(current + delta, 0), length - 1);
}

export function applySelection(
  current: SelectionState,
  visibleIds: readonly string[],
  targetId: string,
  mode: SelectMode,
): SelectionState {
  if (!visibleIds.includes(targetId)) {
    return {
      selectedIds: new Set(current.selectedIds),
      anchorId: current.anchorId,
    };
  }

  if (mode === "replace") {
    return { selectedIds: new Set([targetId]), anchorId: targetId };
  }

  if (mode === "toggle") {
    const selectedIds = new Set(current.selectedIds);
    if (selectedIds.has(targetId)) {
      selectedIds.delete(targetId);
    } else {
      selectedIds.add(targetId);
    }
    return { selectedIds, anchorId: targetId };
  }

  const anchor =
    current.anchorId && visibleIds.includes(current.anchorId)
      ? current.anchorId
      : targetId;
  const start = visibleIds.indexOf(anchor);
  const end = visibleIds.indexOf(targetId);
  const from = Math.min(start, end);
  const to = Math.max(start, end);
  const range = visibleIds.slice(from, to + 1);

  if (mode === "add-range") {
    const selectedIds = new Set(current.selectedIds);
    for (const id of range) {
      selectedIds.add(id);
    }
    return { selectedIds, anchorId: current.anchorId ?? targetId };
  }

  return {
    selectedIds: new Set(range),
    anchorId: current.anchorId ?? targetId,
  };
}
