import { useEffect, useState, type CSSProperties } from "react";

import { addBinding, previewBinding } from "../api/commands";
import { useStore } from "../store";
import { errorMessage } from "../utils/error";
import { ConfirmDialog } from "./ConfirmDialog";

const LARGE_COUNT = 500;

interface BindDialogProps {
  path: string;
  onClose: () => void;
  onToast: (message: string) => void;
}

function parseExtensions(text: string): string[] {
  return text
    .split(",")
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
}

export function BindDialog({ path, onClose, onToast }: BindDialogProps) {
  const [extensionsText, setExtensionsText] = useState(".html, .doc");
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
    const result = await addBinding(
      path,
      recursive,
      autoWatch,
      parseExtensions(extensionsText),
    );
    useStore.setState({
      view: { type: "binding", id: result.binding.id },
    });
    onToast(`已绑定，导入 ${result.added} 个文件`);
    onClose();
  }

  async function handleSubmit() {
    setBusy(true);
    try {
      const preview = await previewBinding(
        path,
        recursive,
        parseExtensions(extensionsText),
      );
      if (preview.count > LARGE_COUNT) {
        setLargeCount(preview.count);
        return;
      }
      await runBind();
    } catch (err) {
      onToast(errorMessage(err, "绑定失败，请确认文件夹仍可访问后重试"));
    } finally {
      setBusy(false);
    }
  }

  async function handleLargeConfirm() {
    setBusy(true);
    try {
      await runBind();
    } catch (err) {
      onToast(errorMessage(err, "绑定失败，请确认文件夹仍可访问后重试"));
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
          <label style={fieldStyle}>
            <span>文件类型</span>
            <input
              type="text"
              value={extensionsText}
              disabled={busy}
              aria-label="文件类型"
              placeholder=".html, .doc"
              onChange={(event) => setExtensionsText(event.target.value)}
              style={inputStyle}
            />
            <span style={hintStyle}>选择需要绑定的文件类型，例如.html, .doc</span>
          </label>
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

const fieldStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 6,
  marginBottom: 12,
};

const inputStyle: CSSProperties = {
  height: 32,
  padding: "0 8px",
  border: "1px solid #e5e7eb",
  borderRadius: 4,
};

const hintStyle: CSSProperties = {
  color: "#6b7280",
  fontSize: 12,
};
