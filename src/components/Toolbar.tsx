import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";

import { importFiles } from "../api/commands";
import { useStore } from "../store";
import type { AppError } from "../types";

const DEFAULT_EXTENSIONS = [".html", ".htm"];

interface ToolbarProps {
  onToast: (message: string) => void;
}

export function Toolbar({ onToast }: ToolbarProps) {
  const extensions = useStore(
    (state) => state.snapshot?.settings.extensions ?? DEFAULT_EXTENSIONS,
  );
  const [busy, setBusy] = useState(false);

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
      <button type="button" disabled={busy} onClick={() => void handleImport()}>
        导入文件
      </button>
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
  return "导入失败";
}
