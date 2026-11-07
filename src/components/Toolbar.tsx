import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";

import { scanAll } from "../api/commands";
import { errorMessage } from "../utils/error";
import { BindDialog } from "./BindDialog";
import { SettingsDialog } from "./SettingsDialog";

interface ToolbarProps {
  onToast: (message: string) => void;
}

export function Toolbar({ onToast }: ToolbarProps) {
  const [busy, setBusy] = useState(false);
  const [bindPath, setBindPath] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);

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
      onToast(errorMessage(err, "扫描失败，请确认绑定的文件夹仍可访问后重试"));
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
