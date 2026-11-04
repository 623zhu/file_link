import { hasManualSource } from "../utils/filter";
import { useStore } from "../store";
import type { View } from "../types";

export function Sidebar() {
  const links = useStore((state) => state.snapshot?.links);
  const view = useStore((state) => state.view);
  const allCount = links?.length ?? 0;
  const manualCount = links?.filter(hasManualSource).length ?? 0;

  return (
    <nav className="sidebar" aria-label="链接来源">
      <SidebarItem
        label="全部链接"
        count={allCount}
        active={view.type === "all"}
        onSelect={() => setView({ type: "all" })}
      />
      <SidebarItem
        label="手动导入"
        count={manualCount}
        active={view.type === "manual"}
        onSelect={() => setView({ type: "manual" })}
      />
      <div className="sidebar-group">
        <div className="sidebar-group-title">文件夹</div>
      </div>
    </nav>
  );
}

function SidebarItem({
  label,
  count,
  active,
  onSelect,
}: {
  label: string;
  count: number;
  active: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      className={active ? "sidebar-item active" : "sidebar-item"}
      aria-current={active ? "page" : undefined}
      onClick={onSelect}
    >
      <span>{label}</span>
      <span className="sidebar-count">{count}</span>
    </button>
  );
}

function setView(view: View) {
  useStore.setState({ view });
}
