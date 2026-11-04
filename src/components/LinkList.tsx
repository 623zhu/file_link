import { useVirtualizer } from "@tanstack/react-virtual";
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
import type { Binding, Link } from "../types";
import { errorMessage } from "../utils/error";
import { filterLinks } from "../utils/filter";
import { ConfirmDialog } from "./ConfirmDialog";
import { LinkRow } from "./LinkRow";

const EMPTY_LINKS: Link[] = [];
const EMPTY_BINDINGS: Binding[] = [];
const ROW_HEIGHT = 36;

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
  const bodyRef = useRef<HTMLDivElement>(null);
  const [confirming, setConfirming] = useState(false);
  // eslint-disable-next-line react-hooks/incompatible-library -- @tanstack/react-virtual
  const virtualizer = useVirtualizer({
    count: visible.length,
    getScrollElement: () => bodyRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 12,
    getItemKey: (index) => visible[index]?.id ?? index,
  });

  useEffect(() => {
    bodyRef.current?.scrollTo(0, 0);
  }, [query, view]);

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
        onToast(
          errorMessage(
            err,
            "打开失败，请确认文件仍在原位置，或从列表中移除该链接",
          ),
        );
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
      onToast(errorMessage(err, "移除链接失败，请稍后重试"));
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
      <div
        className="link-body"
        ref={bodyRef}
        style={{ overflowAnchor: "none" }}
      >
        {visible.length === 0 ? (
          <div className="link-empty">暂无链接</div>
        ) : (
          <div
            style={{
              height: virtualizer.getTotalSize(),
              width: "100%",
              position: "relative",
            }}
          >
            {virtualizer.getVirtualItems().map((virtualRow) => {
              const link = visible[virtualRow.index];
              if (!link) {
                return null;
              }
              return (
                <div
                  key={virtualRow.key}
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    width: "100%",
                    height: virtualRow.size,
                    transform: `translateY(${virtualRow.start}px)`,
                  }}
                >
                  <LinkRow
                    link={link}
                    bindings={bindings}
                    selected={selectedIds.has(link.id)}
                    onSelect={(event) => handleSelect(link.id, event)}
                    onEnsureSelected={() => handleEnsureSelected(link.id)}
                    onRequestRemove={() => setConfirming(true)}
                    onToast={onToast}
                  />
                </div>
              );
            })}
          </div>
        )}
      </div>
      <ConfirmDialog
        open={confirming}
        message={`从 File Link 中移除选中的 ${selectedVisibleIds.length} 个链接？只删除链接，不删除原文件。`}
        onConfirm={() => void handleRemoveSelected()}
        onCancel={() => setConfirming(false)}
      />
    </section>
  );
}
