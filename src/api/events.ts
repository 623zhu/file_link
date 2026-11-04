import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  AutoImportedPayload,
  BindingUnavailablePayload,
  DataRecoveredPayload,
} from "../types";

export function onStateChanged(handler: () => void): Promise<UnlistenFn> {
  return listen("state-changed", () => {
    handler();
  });
}

export function onAutoImported(
  handler: (payload: AutoImportedPayload) => void,
): Promise<UnlistenFn> {
  return listen<AutoImportedPayload>("auto-imported", (event) => {
    handler(event.payload);
  });
}

export function onBindingUnavailable(
  handler: (payload: BindingUnavailablePayload) => void,
): Promise<UnlistenFn> {
  return listen<BindingUnavailablePayload>("binding-unavailable", (event) => {
    handler(event.payload);
  });
}

export function onDataRecovered(
  handler: (payload: DataRecoveredPayload) => void,
): Promise<UnlistenFn> {
  return listen<DataRecoveredPayload>("data-recovered", (event) => {
    handler(event.payload);
  });
}

export async function subscribeWatcherEvents(handlers: {
  onStateChanged: () => void;
  onAutoImported: (payload: AutoImportedPayload) => void;
  onBindingUnavailable: (payload: BindingUnavailablePayload) => void;
}): Promise<UnlistenFn> {
  const unlistens = await Promise.all([
    onStateChanged(handlers.onStateChanged),
    onAutoImported(handlers.onAutoImported),
    onBindingUnavailable(handlers.onBindingUnavailable),
  ]);
  return () => {
    for (const unlisten of unlistens) {
      unlisten();
    }
  };
}
