import { useEffect, useState } from "react";

import { openLink, removeLinks, revealLink } from "../api/commands";
import type { AppError, Binding, Link, Source } from "../types";
import { ConfirmDialog } from "./ConfirmDialog";

interface LinkRowProps {
  link: Link;
  bindings: Binding[];
  onToast: (message: string) => void;
}

interface MenuPosition {
  x: number;
  y: number;
}

export function LinkRow({ link, bindings, onToast }: LinkRowProps) {
  const [menu, setMenu] = useState<MenuPosition | null>(null);
  const [confirming, setConfirming] = useState(false);

  useEffect(() => {
    if (!menu) {
      return;
    }
    function close() {
      setMenu(null);
    }
    window.addEventListener("click", close);
    window.addEventListener("contextmenu", close, true);
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("contextmenu", close, true);
    };
  }, [menu]);

  async function handleOpen() {
    setMenu(null);
    try {
      await openLink(link.id);
    } catch (err) {
      onToast(errorMessage(err));
    }
  }

  async function handleReveal() {
    setMenu(null);
    try {
      await revealLink(link.id);
    } catch (err) {
      onToast(errorMessage(err));
    }
  }

  async function handleCopyPath() {
    setMenu(null);
    try {
      await copyText(link.path);
      onToast("已复制路径");
    } catch {
      onToast("复制路径失败");
    }
  }

  async function handleRemove() {
    setConfirming(false);
    try {
      await removeLinks([link.id]);
    } catch (err) {
      onToast(errorMessage(err));
    }
  }

  const source = sourceLabel(link.sources, bindings);

  return (
    <div
      className={link.missing ? "link-row missing" : "link-row"}
      onDoubleClick={() => void handleOpen()}
      onContextMenu={(event) => {
        event.preventDefault();
        event.stopPropagation();
        setMenu({
          x: Math.min(event.clientX, window.innerWidth - 200),
          y: Math.min(event.clientY, window.innerHeight - 180),
        });
      }}
    >
      <div className="link-cell link-name">
        <span className="link-name-text" title={link.name}>
          {link.name}
        </span>
        {link.missing ? <span className="missing-badge">缺失</span> : null}
      </div>
      <div className="link-cell" title={link.path}>
        {link.path}
      </div>
      <div className="link-cell" title={source}>
        {source}
      </div>
      <div className="link-cell">{formatAddedAt(link.addedAt)}</div>
      {menu ? (
        <div
          className="context-menu"
          style={{ left: menu.x, top: menu.y }}
          role="menu"
        >
          <button
            type="button"
            role="menuitem"
            onClick={() => void handleOpen()}
          >
            打开
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={() => void handleReveal()}
          >
            在资源管理器中显示
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={() => void handleCopyPath()}
          >
            复制路径
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              setMenu(null);
              setConfirming(true);
            }}
          >
            移除链接
          </button>
        </div>
      ) : null}
      <ConfirmDialog
        open={confirming}
        message="从 File Link 中移除选中的 1 个链接？只删除链接，不会删除磁盘上的原文件。"
        onConfirm={() => void handleRemove()}
        onCancel={() => setConfirming(false)}
      />
    </div>
  );
}

function sourceLabel(sources: Source[], bindings: Binding[]): string {
  const labels: string[] = [];
  for (const source of sources) {
    if (source.type === "manual") {
      labels.push("手动");
      continue;
    }
    const binding = bindings.find((item) => item.id === source.bindingId);
    labels.push(folderName(binding?.path) ?? "文件夹");
  }
  return labels.join(" ");
}

function folderName(path: string | undefined): string | undefined {
  if (!path) {
    return undefined;
  }
  const parts = path.split(/[\\/]/).filter((part) => part.length > 0);
  return parts[parts.length - 1];
}

function formatAddedAt(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  const hours = String(date.getHours()).padStart(2, "0");
  const minutes = String(date.getMinutes()).padStart(2, "0");
  return `${month}-${day} ${hours}:${minutes}`;
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

async function copyText(value: string): Promise<void> {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(value);
    return;
  }
  const textarea = document.createElement("textarea");
  textarea.value = value;
  textarea.setAttribute("readonly", "");
  textarea.style.position = "fixed";
  textarea.style.left = "-9999px";
  document.body.appendChild(textarea);
  textarea.select();
  const copied = document.execCommand("copy");
  textarea.remove();
  if (!copied) {
    throw new Error("copy failed");
  }
}
