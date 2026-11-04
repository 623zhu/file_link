import { create } from "zustand";

import { getSnapshot } from "./api/commands";
import type { Snapshot, View } from "./types";

interface Store {
  snapshot: Snapshot | null;
  view: View;
  query: string;
  selectedIds: Set<string>;
  refresh: () => Promise<void>;
}

export const useStore = create<Store>((set) => ({
  snapshot: null,
  view: { type: "all" },
  query: "",
  selectedIds: new Set(),
  refresh: async () => {
    const snapshot = await getSnapshot();
    set({ snapshot });
  },
}));
