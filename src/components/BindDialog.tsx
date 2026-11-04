import { useEffect, useState, type CSSProperties } from "react";

import { addBinding, previewBinding } from "../api/commands";
import type { AppError } from "../types";
import { ConfirmDialog } from "./ConfirmDialog";

const LARGE_COUNT = 500;

interface BindDialogProps {
  path: string;
  onClose: () => void;
  onToast: (message: string) => void;
}

export function BindDialog({ path, onClose, onToast }: BindDialogProps) {
  const [recursive, setRecursive] = useState(true);
  const [autoWatch, setAutoWatch] = useState(true);
  const [busy, setBusy] = useState(false);
  const [largeCount, setLargeCount] = useState<number | null>(null);

  useEffect(() => {
    if (largeCount != null) {
      return;
    }
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape" && !busy) {
        onClose();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [busy, largeCount, onClose]);

  async function runBind() {
    const result = await addBinding(path, recursive, autoWatch);
    onToast(`已绑定，导入 ${result.added} 个文件`);
    onClose();
  }

  async function handleSubmit() {
    setBusy(true);
    try {
      const preview = await previewBinding(path, recursive);
      if (preview.count > LARGE_COUNT) {
        setLargeCount(preview.count);
        return;
      }
      await runBind();
    } catch (err) {
      onToast(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleLargeConfirm() {
    setBusy(true);
    try {
      await runBind();
    } catch (err) {
      onToast(errorMessage(err));
    } finally {
      setBusy(false);
      setLargeCount(null);
    }
  }

  return (
    <>
      <div
        className="dialog-backdrop"
        role="presentation"
        onClick={() => {
          if (!busy && largeCount == null) {
            onClose();
          }
        }}
      >
        <div
          className="dialog"
          role="dialog"
          aria-modal="true"
          aria-label="绑定文件夹"
          onClick={(event) => event.stopPropagation()}
        >
          <p className="dialog-message" title={path}>
            {path}
          </p>
          <label style={checkboxStyle}>
            <input
              type="checkbox"
              checked={recursive}
              disabled={busy}
              onChange={(event) => setRecursive(event.target.checked)}
            />
            包含子文件夹
          </label>
          <label style={checkboxStyle}>
            <input
              type="checkbox"
              checked={autoWatch}
              disabled={busy}
              onChange={(event) => setAutoWatch(event.target.checked)}
            />
            自动导入
          </label>
          <div className="dialog-actions">
            <button type="button" disabled={busy} onClick={onClose}>
              取消
            </button>
            <button
              type="button"
              className="dialog-confirm"
              disabled={busy}
              onClick={() => void handleSubmit()}
            >
              {busy ? "处理中…" : "绑定"}
            </button>
          </div>
        </div>
      </div>
      <ConfirmDialog
        open={largeCount != null}
        message={`该文件夹匹配 ${largeCount ?? 0} 个文件，确定绑定？`}
        confirmLabel="绑定"
        onConfirm={() => void handleLargeConfirm()}
        onCancel={() => setLargeCount(null)}
      />
    </>
  );
}

const checkboxStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 8,
  marginBottom: 10,
};

function errorMessage(err: unknown): string {
  if (err && typeof err === "object" && "message" in err) {
    const message = (err as AppError).message;
    if (typeof message === "string" && message.length > 0) {
      return message;
    }
  }
  return "绑定失败";
}
