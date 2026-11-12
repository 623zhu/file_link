import { useEffect, useState } from "react";
import { createPortal } from "react-dom";

import { openLink, revealLink, setStarred } from "../api/commands";
import type { Link } from "../types";
import { errorMessage } from "../utils/error";

interface LinkRowProps {
  link: Link;
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
          "打开失败，请确认文件仍在原位置",
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

  async function handleStar() {
    setMenu(null);
    try {
      await setStarred(link.id, !link.starred);
    } catch (err) {
      onToast(errorMessage(err, "更新星标失败，请稍后重试"));
    }
  }

  const className = ["link-row", selected ? "selected" : ""]
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
        if ((event.target as HTMLElement).closest(".context-menu, .link-star")) {
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
          y: Math.min(event.clientY, window.innerHeight - 220),
        });
      }}
    >
      <button
        type="button"
        className={link.starred ? "link-star starred" : "link-star"}
        aria-label={link.starred ? "取消星标" : "标星"}
        aria-pressed={link.starred}
        onClick={(event) => {
          event.stopPropagation();
          void handleStar();
        }}
        onDoubleClick={(event) => event.stopPropagation()}
      >
        <StarIcon filled={link.starred} />
      </button>
      <div className="link-cell link-name">
        <span className="link-name-text" title={link.name}>
          {link.name}
        </span>
      </div>
      <div className="link-cell" title={link.path}>
        {link.path}
      </div>
      <div className="link-cell">{formatModifiedAt(link.modifiedAt)}</div>
      {menu
        ? createPortal(
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
                onClick={() => void handleStar()}
              >
                {link.starred ? "取消星标" : "标星"}
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
            </div>,
            document.body,
          )
        : null}
    </div>
  );
}

function StarIcon({ filled }: { filled: boolean }) {
  return (
    <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
      <path
        d="M8 1.4 9.94 5.33l4.36.37-3.32 2.87.99 4.26L8 10.72 4.03 12.83l.99-4.26-3.32-2.87 4.36-.37Z"
        fill={filled ? "currentColor" : "none"}
        stroke="currentColor"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function formatModifiedAt(value: string): string {
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
