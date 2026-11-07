import { invoke } from "@tauri-apps/api/core";

import type {
  AddBindingResult,
  Binding,
  ImportResult,
  PreviewBindingResult,
  ReconcileSummary,
  RemoveBindingResult,
  RemoveLinksResult,
  Settings,
  Snapshot,
} from "../types";

export function getSnapshot(): Promise<Snapshot> {
  return invoke("get_snapshot");
}

export function importFiles(paths: string[]): Promise<ImportResult> {
  return invoke("import_files", { paths });
}

export function previewBinding(
  path: string,
  recursive: boolean,
  extensions: string[],
): Promise<PreviewBindingResult> {
  return invoke("preview_binding", { path, recursive, extensions });
}

export function addBinding(
  path: string,
  recursive: boolean,
  autoWatch: boolean,
  extensions: string[],
): Promise<AddBindingResult> {
  return invoke("add_binding", { path, recursive, autoWatch, extensions });
}

export function updateBinding(
  id: string,
  changes: {
    recursive?: boolean;
    autoWatch?: boolean;
    extensions?: string[];
  } = {},
): Promise<Binding> {
  return invoke("update_binding", { id, ...changes });
}

export function removeBinding(id: string): Promise<RemoveBindingResult> {
  return invoke("remove_binding", { id });
}

export function scanBinding(id: string): Promise<ReconcileSummary> {
  return invoke("scan_binding", { id });
}

export function scanAll(): Promise<ReconcileSummary> {
  return invoke("scan_all");
}

export function removeLinks(ids: string[]): Promise<RemoveLinksResult> {
  return invoke("remove_links", { ids });
}

export function openLink(id: string): Promise<void> {
  return invoke("open_link", { id });
}

export function revealLink(id: string): Promise<void> {
  return invoke("reveal_link", { id });
}

export function updateSettings(extensions: string[]): Promise<Settings> {
  return invoke("update_settings", { extensions });
}
