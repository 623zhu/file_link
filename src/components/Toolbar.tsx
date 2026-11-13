import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useRef, useState } from "react";

import { scanAll } from "../api/commands";
import { useStore } from "../store";
import { errorMessage } from "../utils/error";
import { scanSummaryMessage } from "../utils/scan";
import { BindDialog } from "./BindDialog";

interface ToolbarProps {
  onToast: (message: string) => void;
}

export function Toolbar({ onToast }: ToolbarProps) {
  const query = useStore((state) => state.query);
  const searchRef = useRef<HTMLInputElement>(null);
  const [busy, setBusy] = useState(false);
  const [bindPath, setBindPath] = useState<string | null>(null);

  useEffect(() => {
    function handleKeyDown(event: globalThis.KeyboardEvent) {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "f") {
        event.preventDefault();
        searchRef.current?.focus();
        searchRef.current?.select();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

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
      onToast(scanSummaryMessage(summary));
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
      <input
        ref={searchRef}
        type="search"
        value={query}
        placeholder="在所有文件夹中搜索文件名或路径……"
        aria-label="在所有文件夹中搜索文件名或路径"
        onChange={(event) => useStore.setState({ query: event.target.value })}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            useStore.setState({ query: "" });
          } else if (event.key === "ArrowDown") {
            event.preventDefault();
            useStore.setState((state) => ({
              listFocusRequest: state.listFocusRequest + 1,
            }));
          }
        }}
        style={{
          flex: 1,
          minWidth: 160,
          height: 28,
          padding: "0 8px",
          border: "1px solid #e5e7eb",
          borderRadius: 4,
        }}
      />
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
      {bindPath ? (
        <BindDialog
          path={bindPath}
          onClose={() => setBindPath(null)}
          onToast={onToast}
        />
      ) : null}
    </header>
  );
}
