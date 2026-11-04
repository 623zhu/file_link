import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";

import { importFiles, scanAll } from "../api/commands";
import { useStore } from "../store";
import type { AppError } from "../types";
import { BindDialog } from "./BindDialog";
import { SettingsDialog } from "./SettingsDialog";

const DEFAULT_EXTENSIONS = [".html", ".htm"];

interface ToolbarProps {
  onToast: (message: string) => void;
}

export function Toolbar({ onToast }: ToolbarProps) {
  const extensions = useStore(
    (state) => state.snapshot?.settings.extensions ?? DEFAULT_EXTENSIONS,
  );
  const query = useStore((state) => state.query);
  const [busy, setBusy] = useState(false);
  const [bindPath, setBindPath] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);

  async function handleImport() {
    const selected = await open({
      multiple: true,
      title: "导入文件",
      filters: dialogFilters(extensions),
    });
    if (selected == null) {
      return;
    }

    const paths = Array.isArray(selected) ? selected : [selected];
    setBusy(true);
    try {
      const result = await importFiles(paths);
      onToast(`已导入 ${result.added} 个，跳过 ${result.skipped} 个已存在`);
    } catch (err) {
      onToast(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleBind() {
    const selected = await open({
      directory: true,
      title: "绑定文件夹",
    });
    if (typeof selected !== "string") {
      return;
    }
    setBindPath(selected);
  }

  async function handleScanAll() {
    setBusy(true);
    try {
      const summary = await scanAll();
      onToast(
        `新增 ${summary.added}，恢复 ${summary.restored}，缺失 ${summary.missing}`,
      );
    } catch (err) {
      onToast(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <header
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        padding: "12px 16px",
        borderBottom: "1px solid #e5e7eb",
      }}
    >
      <input
        type="search"
        value={query}
        placeholder="搜索文件名或路径……"
        aria-label="搜索文件名或路径"
        onChange={(event) => useStore.setState({ query: event.target.value })}
        style={{
          flex: 1,
          minWidth: 160,
          height: 28,
          padding: "0 8px",
          border: "1px solid #e5e7eb",
          borderRadius: 4,
        }}
      />
      <button type="button" disabled={busy} onClick={() => void handleImport()}>
        导入文件
      </button>
      <button type="button" disabled={busy} onClick={() => void handleBind()}>
        绑定文件夹
      </button>
      <button
        type="button"
        disabled={busy}
        onClick={() => void handleScanAll()}
      >
        扫描新文件
      </button>
      <button
        type="button"
        disabled={busy}
        title="设置"
        aria-label="设置"
        onClick={() => setSettingsOpen(true)}
      >
        ⚙
      </button>
      {bindPath ? (
        <BindDialog
          path={bindPath}
          onClose={() => setBindPath(null)}
          onToast={onToast}
        />
      ) : null}
      {settingsOpen ? (
        <SettingsDialog
          onClose={() => setSettingsOpen(false)}
          onToast={onToast}
        />
      ) : null}
    </header>
  );
}

function dialogFilters(extensions: string[]) {
  const names = extensions
    .map((ext) => ext.replace(/^\./, ""))
    .filter((ext) => ext.length > 0);
  const filters = [];
  if (names.length > 0) {
    filters.push({
      name: names.map((name) => name.toUpperCase()).join(" / "),
      extensions: names,
    });
  }
  filters.push({ name: "所有文件", extensions: ["*"] });
  return filters;
}

function errorMessage(err: unknown): string {
  if (err && typeof err === "object" && "message" in err) {
    const message = (err as AppError).message;
    if (typeof message === "string" && message.length > 0) {
      return message;
    }
  }
  return "操作失败";
}
