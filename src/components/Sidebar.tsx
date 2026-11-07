import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState, type MouseEvent } from "react";

import { removeBinding, scanBinding, updateBinding } from "../api/commands";
import {
  browseTabFromView,
  useStore,
  viewForBrowseTab,
  type BrowseTab,
} from "../store";
import type { Binding, View } from "../types";
import { errorMessage } from "../utils/error";
import {
  extensionLabel,
  hasAnyBindingSource,
  hasBindingSource,
  linkExtension,
  listedExtensions,
} from "../utils/filter";
import { folderName, isNetworkPath, NETWORK_WATCH_HINT } from "../utils/path";
import { ConfirmDialog } from "./ConfirmDialog";

interface SidebarProps {
  onToast: (message: string) => void;
}

interface MenuState {
  binding: Binding;
  x: number;
  y: number;
}

const UNBIND_MESSAGE =
  "解除绑定后，将移除由该文件夹自动生成的链接。只删除链接，不删除原文件。";

export function Sidebar({ onToast }: SidebarProps) {
  const links = useStore((state) => state.snapshot?.links);
  const bindings = useStore((state) => state.snapshot?.bindings) ?? [];
  const view = useStore((state) => state.view);
  const extensions = listedExtensions(links ?? []);
  const [emptyTab, setEmptyTab] = useState<BrowseTab>("folder");
  const tab = view.type === "none" ? emptyTab : browseTabFromView(view);
  const [lastBindingId, setLastBindingId] = useState(
    view.type === "binding" ? view.id : undefined,
  );
  const [lastExt, setLastExt] = useState(
    view.type === "extension" ? view.ext : undefined,
  );
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [unbinding, setUnbinding] = useState<Binding | null>(null);

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

  async function handleScan(binding: Binding) {
    setMenu(null);
    try {
      const summary = await scanBinding(binding.id);
      onToast(
        `新增 ${summary.added}，恢复 ${summary.restored}，缺失 ${summary.missing}`,
      );
    } catch (err) {
      onToast(errorMessage(err, "扫描失败，请确认文件夹仍可访问后重试"));
    }
  }

  async function handleToggleAutoWatch(binding: Binding) {
    setMenu(null);
    const next = !binding.autoWatch;
    try {
      await updateBinding(binding.id, { autoWatch: next });
      if (next && isNetworkPath(binding.path)) {
        onToast(NETWORK_WATCH_HINT);
      }
    } catch (err) {
      onToast(errorMessage(err, "更新绑定失败，请稍后重试"));
    }
  }

  async function handleToggleRecursive(binding: Binding) {
    setMenu(null);
    try {
      await updateBinding(binding.id, { recursive: !binding.recursive });
    } catch (err) {
      onToast(errorMessage(err, "更新绑定失败，请稍后重试"));
    }
  }

  async function handleOpenFolder(binding: Binding) {
    setMenu(null);
    try {
      await invoke("plugin:opener|open_path", {
        path: binding.path,
        with: null,
      });
    } catch (err) {
      onToast(
        errorMessage(err, "无法在资源管理器中打开该文件夹，请确认目录仍存在"),
      );
    }
  }

  function selectTab(next: BrowseTab) {
    if (next === tab) {
      return;
    }
    const nextView = viewForBrowseTab(
      next,
      bindings,
      extensions,
      lastBindingId,
      lastExt,
    );
    if (nextView.type === "none") {
      setEmptyTab(next);
    } else if (nextView.type === "binding") {
      setLastBindingId(nextView.id);
    } else {
      setLastExt(nextView.ext);
    }
    setView(nextView);
  }

  function selectBinding(id: string) {
    setLastBindingId(id);
    setView({ type: "binding", id });
  }

  function selectExtension(ext: string) {
    setLastExt(ext);
    setView({ type: "extension", ext });
  }

  async function handleUnbind() {
    if (!unbinding) {
      return;
    }
    const id = unbinding.id;
    setUnbinding(null);
    try {
      await removeBinding(id);
    } catch (err) {
      onToast(errorMessage(err, "解除绑定失败，请稍后重试"));
    }
  }

  return (
    <nav className="sidebar" aria-label="链接来源">
      <div className="sidebar-tabs" role="tablist" aria-label="浏览方式">
        <button
          type="button"
          className={tab === "folder" ? "sidebar-tab active" : "sidebar-tab"}
          role="tab"
          aria-selected={tab === "folder"}
          onClick={() => selectTab("folder")}
        >
          文件夹
        </button>
        <button
          type="button"
          className={tab === "type" ? "sidebar-tab active" : "sidebar-tab"}
          role="tab"
          aria-selected={tab === "type"}
          onClick={() => selectTab("type")}
        >
          类型
        </button>
      </div>
      <div className="sidebar-group" role="tabpanel">
        {tab === "folder"
          ? bindings.map((binding) => (
              <div key={binding.id}>
                <SidebarItem
                  label={folderName(binding.path)}
                  count={
                    links?.filter((link) => hasBindingSource(link, binding.id))
                      .length ?? 0
                  }
                  active={view.type === "binding" && view.id === binding.id}
                  unavailable={binding.unavailable}
                  title={binding.path}
                  onSelect={() => selectBinding(binding.id)}
                  onContextMenu={(event) => {
                    event.preventDefault();
                    event.stopPropagation();
                    selectBinding(binding.id);
                    setMenu({
                      binding,
                      x: Math.min(event.clientX, window.innerWidth - 220),
                      y: Math.min(event.clientY, window.innerHeight - 220),
                    });
                  }}
                />
                {binding.autoWatch &&
                isNetworkPath(binding.path) &&
                !binding.unavailable ? (
                  <p className="sidebar-hint">{NETWORK_WATCH_HINT}</p>
                ) : null}
              </div>
            ))
          : extensions.map((ext) => (
              <SidebarItem
                key={ext}
                label={extensionLabel(ext)}
                count={
                  links?.filter(
                    (item) =>
                      hasAnyBindingSource(item) && linkExtension(item) === ext,
                  ).length ?? 0
                }
                active={view.type === "extension" && view.ext === ext}
                title={ext}
                onSelect={() => selectExtension(ext)}
              />
            ))}
      </div>
      {menu ? (
        <div
          className="context-menu"
          style={{ left: menu.x, top: menu.y }}
          role="menu"
        >
          <button
            type="button"
            role="menuitem"
            onClick={() => void handleScan(menu.binding)}
          >
            扫描
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={() => void handleToggleAutoWatch(menu.binding)}
          >
            {menu.binding.autoWatch ? "✓ 自动导入" : "自动导入"}
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={() => void handleToggleRecursive(menu.binding)}
          >
            {menu.binding.recursive ? "✓ 包含子文件夹" : "包含子文件夹"}
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={() => void handleOpenFolder(menu.binding)}
          >
            在资源管理器中打开
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              setUnbinding(menu.binding);
              setMenu(null);
            }}
          >
            解除绑定
          </button>
        </div>
      ) : null}
      <ConfirmDialog
        open={unbinding != null}
        message={UNBIND_MESSAGE}
        confirmLabel="解除绑定"
        onConfirm={() => void handleUnbind()}
        onCancel={() => setUnbinding(null)}
      />
    </nav>
  );
}

function SidebarItem({
  label,
  count,
  active,
  unavailable,
  title,
  onSelect,
  onContextMenu,
}: {
  label: string;
  count: number;
  active: boolean;
  unavailable?: boolean;
  title?: string;
  onSelect: () => void;
  onContextMenu?: (event: MouseEvent<HTMLButtonElement>) => void;
}) {
  const className = [
    "sidebar-item",
    active ? "active" : "",
    unavailable ? "unavailable" : "",
  ]
    .filter(Boolean)
    .join(" ");
  return (
    <button
      type="button"
      className={className}
      aria-current={active ? "page" : undefined}
      title={title}
      onClick={onSelect}
      onContextMenu={onContextMenu}
    >
      <span
        style={{
          minWidth: 0,
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
        }}
      >
        {label}
      </span>
      <span className="sidebar-item-end">
        {unavailable ? (
          <span className="sidebar-status">目录不可用</span>
        ) : null}
        <span className="sidebar-count">{count}</span>
      </span>
    </button>
  );
}

function setView(view: View) {
  useStore.setState({ view });
}
