import { useEffect, useState, type CSSProperties } from "react";

import { updateBinding } from "../api/commands";
import { useStore } from "../store";
import { errorMessage } from "../utils/error";

interface SettingsDialogProps {
  bindingId: string;
  onClose: () => void;
  onToast: (message: string) => void;
}

export function SettingsDialog({
  bindingId,
  onClose,
  onToast,
}: SettingsDialogProps) {
  const extensions = useStore((state) => {
    const binding = state.snapshot?.bindings.find(
      (item) => item.id === bindingId,
    );
    return binding?.extensions ?? [".html"];
  });
  const [text, setText] = useState(extensions.join(", "));
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape" && !busy) {
        onClose();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [busy, onClose]);

  async function handleSave() {
    const items = text
      .split(",")
      .map((item) => item.trim())
      .filter((item) => item.length > 0);
    setBusy(true);
    try {
      await updateBinding(bindingId, { extensions: items });
      onToast("新规则将在之后的扫描中生效");
      onClose();
    } catch (err) {
      onToast(errorMessage(err, "保存失败，请检查填写内容后重试"));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div
      className="dialog-backdrop"
      role="presentation"
      onClick={() => {
        if (!busy) {
          onClose();
        }
      }}
    >
      <div
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-label="设置"
        onClick={(event) => event.stopPropagation()}
      >
        <p className="dialog-message">文件类型</p>
        <label style={fieldStyle}>
          <input
            type="text"
            value={text}
            disabled={busy}
            aria-label="文件类型"
            placeholder=".html"
            onChange={(event) => setText(event.target.value)}
            style={inputStyle}
          />
          <span style={hintStyle}>选择需要绑定的文件类型，例如.html</span>
        </label>
        <div className="dialog-actions">
          <button type="button" disabled={busy} onClick={onClose}>
            取消
          </button>
          <button
            type="button"
            className="dialog-confirm"
            disabled={busy}
            onClick={() => void handleSave()}
          >
            {busy ? "保存中…" : "保存"}
          </button>
        </div>
      </div>
    </div>
  );
}

const fieldStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 6,
  marginBottom: 16,
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
