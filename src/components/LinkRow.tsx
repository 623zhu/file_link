import { useEffect, useState } from "react";

import { openLink, revealLink } from "../api/commands";
import type { Binding, Link, Source } from "../types";
import { errorMessage } from "../utils/error";

interface LinkRowProps {
  link: Link;
  bindings: Binding[];
  selected: boolean;
  onSelect: (event: {
    shiftKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
  }) => void;
  onEnsureSelected: () => void;
  onRequestRemove: () => void;
  onToast: (message: string) => void;
}

interface MenuPosition {
  x: number;
  y: number;
}

export function LinkRow({
  link,
  bindings,
  selected,
  onSelect,
  onEnsureSelected,
  onRequestRemove,
  onToast,
}: LinkRowProps) {
  const [menu, setMenu] = useState<MenuPosition | null>(null);

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
      onToast(
        errorMessage(
          err,
          "打开失败，请确认文件仍在原位置，或从列表中移除该链接",
        ),
      );
    }
  }

  async function handleReveal() {
    setMenu(null);
    try {
      await revealLink(link.id);
    } catch (err) {
      onToast(
        errorMessage(err, "无法在资源管理器中显示该文件，请确认文件仍在原位置"),
      );
    }
  }

  async function handleCopyPath() {
    setMenu(null);
    try {
      await copyText(link.path);
      onToast("已复制路径");
    } catch {
      onToast("复制路径失败，请重试或手动选择路径复制");
    }
  }

  const source = sourceLabel(link.sources, bindings);
  const className = [
    "link-row",
    link.missing ? "missing" : "",
    selected ? "selected" : "",
  ]
    .filter((part) => part.length > 0)
    .join(" ");

  return (
    <div
      className={className}
      data-link-id={link.id}
      aria-selected={selected}
      onClick={(event) => {
        if (event.detail > 1) {
          return;
        }
        if ((event.target as HTMLElement).closest(".context-menu")) {
          return;
        }
        onSelect(event);
      }}
      onDoubleClick={() => void handleOpen()}
      onContextMenu={(event) => {
        event.preventDefault();
        event.stopPropagation();
        onEnsureSelected();
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
              onRequestRemove();
            }}
          >
            移除链接
          </button>
        </div>
      ) : null}
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
    throw new Error("复制失败");
  }
}
