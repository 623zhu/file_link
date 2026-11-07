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
  refresh: () => Promise<void>;
}

export const useStore = create<Store>((set) => ({
  snapshot: null,
  view: { type: "none" },
  selectedIds: new Set(),
  refresh: async () => {
    const snapshot = await getSnapshot();
    set((state) => ({
      snapshot,
      view: resolveView(state.view, snapshot.bindings, snapshot.links),
    }));
  },
}));

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
