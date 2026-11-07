export type Source =
  { type: "manual" } | { type: "binding"; bindingId: string };

export interface Link {
  id: string;
  path: string;
  key: string;
  name: string;
  sources: Source[];
  modifiedAt: string;
  missing: boolean;
}

export interface Binding {
  id: string;
  path: string;
  key: string;
  recursive: boolean;
  autoWatch: boolean;
  extensions: string[];
  ruleId: string | null;
  createdAt: string;
  unavailable: boolean;
}

export interface Settings {
  extensions: string[];
}

export interface Snapshot {
  settings: Settings;
  bindings: Binding[];
  links: Link[];
}

export interface AppError {
  code: string;
  message: string;
}

export type View =
  | { type: "binding"; id: string }
  | { type: "extension"; ext: string }
  | { type: "none" };

export interface ImportResult {
  added: number;
  skipped: number;
}

export interface PreviewBindingResult {
  count: number;
}

export interface AddBindingResult {
  binding: Binding;
  added: number;
}

export interface RemoveBindingResult {
  removedLinks: number;
}

export interface ReconcileSummary {
  added: number;
  restored: number;
  missing: number;
}

export interface RemoveLinksResult {
  removed: number;
}

export interface AutoImportedPayload {
  bindingId: string;
  added: number;
}

export interface BindingUnavailablePayload {
  bindingId: string;
}

export interface DataRecoveredPayload {
  backupPath: string;
}
