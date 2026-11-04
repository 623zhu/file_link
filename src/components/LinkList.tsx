import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";

import { openLink, removeLinks } from "../api/commands";
import {
  applySelection,
  selectionModeFromEvent,
  useStore,
  type SelectMode,
} from "../store";
import type { AppError, Binding, Link } from "../types";
import { filterLinks } from "../utils/filter";
import { ConfirmDialog } from "./ConfirmDialog";
import { LinkRow } from "./LinkRow";

const EMPTY_LINKS: Link[] = [];
const EMPTY_BINDINGS: Binding[] = [];

interface LinkListProps {
  onToast: (message: string) => void;
}

export function LinkList({ onToast }: LinkListProps) {
  const links = useStore((state) => state.snapshot?.links) ?? EMPTY_LINKS;
  const bindings =
    useStore((state) => state.snapshot?.bindings) ?? EMPTY_BINDINGS;
  const view = useStore((state) => state.view);
  const query = useStore((state) => state.query);
  const selectedIds = useStore((state) => state.selectedIds);
  const visible = useMemo(
    () => filterLinks(links, view, query),
    [links, view, query],
  );
  const visibleIds = useMemo(() => visible.map((link) => link.id), [visible]);
  const anchorIdRef = useRef<string | null>(null);
  const listRef = useRef<HTMLElement>(null);
  const [confirming, setConfirming] = useState(false);

  useEffect(() => {
    const visibleSet = new Set(visibleIds);
    const current = useStore.getState().selectedIds;
    const next = new Set<string>();
    for (const id of current) {
      if (visibleSet.has(id)) {
        next.add(id);
      }
    }
    if (next.size !== current.size) {
      useStore.setState({ selectedIds: next });
    }
    if (anchorIdRef.current && !visibleSet.has(anchorIdRef.current)) {
      anchorIdRef.current = next.size > 0 ? [...next][next.size - 1] : null;
    }
  }, [visibleIds]);

  const selectedVisibleIds = visibleIds.filter((id) => selectedIds.has(id));

  function applySelect(id: string, mode: SelectMode) {
    const current = {
      selectedIds: useStore.getState().selectedIds,
      anchorId: anchorIdRef.current,
    };
    const next = applySelection(current, visibleIds, id, mode);
    anchorIdRef.current = next.anchorId;
    useStore.setState({ selectedIds: next.selectedIds });
    listRef.current?.focus();
  }

  function handleSelect(
    id: string,
    event: { shiftKey: boolean; ctrlKey: boolean; metaKey: boolean },
  ) {
    applySelect(id, selectionModeFromEvent(event));
  }

  function handleEnsureSelected(id: string) {
    if (!useStore.getState().selectedIds.has(id)) {
      applySelect(id, "replace");
    } else {
      listRef.current?.focus();
    }
  }

  async function handleOpenSelected() {
    if (selectedVisibleIds.length === 0) {
      return;
    }
    for (const id of selectedVisibleIds) {
      try {
        await openLink(id);
      } catch (err) {
        onToast(errorMessage(err));
      }
    }
  }

  async function handleRemoveSelected() {
    setConfirming(false);
    const ids = visibleIds.filter((id) =>
      useStore.getState().selectedIds.has(id),
    );
    if (ids.length === 0) {
      return;
    }
    try {
      await removeLinks(ids);
      const remain = new Set(useStore.getState().selectedIds);
      for (const id of ids) {
        remain.delete(id);
      }
      useStore.setState({ selectedIds: remain });
    } catch (err) {
      onToast(errorMessage(err));
    }
  }

  function handleKeyDown(event: KeyboardEvent<HTMLElement>) {
    if (confirming) {
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      void handleOpenSelected();
      return;
    }
    if (event.key === "Delete") {
      event.preventDefault();
      if (selectedVisibleIds.length > 0) {
        setConfirming(true);
      }
    }
  }

  return (
    <section
      ref={listRef}
      className="link-list"
      aria-label="链接列表"
      tabIndex={0}
      onKeyDown={handleKeyDown}
    >
      <div className="link-header">
        <div>名称</div>
        <div>路径</div>
        <div>来源</div>
        <div>加入时间</div>
      </div>
      <div className="link-body">
        {visible.length === 0 ? (
          <div className="link-empty">暂无链接</div>
        ) : (
          visible.map((link) => (
            <LinkRow
              key={link.id}
              link={link}
              bindings={bindings}
              selected={selectedIds.has(link.id)}
              onSelect={(event) => handleSelect(link.id, event)}
              onEnsureSelected={() => handleEnsureSelected(link.id)}
              onRequestRemove={() => setConfirming(true)}
              onToast={onToast}
            />
          ))
        )}
      </div>
      <ConfirmDialog
        open={confirming}
        message={`从 File Link 中移除选中的 ${selectedVisibleIds.length} 个链接？只删除链接，不会删除磁盘上的原文件。`}
        onConfirm={() => void handleRemoveSelected()}
        onCancel={() => setConfirming(false)}
      />
    </section>
  );
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
