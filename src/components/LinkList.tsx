import { useVirtualizer } from "@tanstack/react-virtual";
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";

import { openLink, removeLinks, setStarred } from "../api/commands";
import {
  applySelection,
  nextCursorIndex,
  selectionModeFromEvent,
  useStore,
  type SelectMode,
} from "../store";
import type { Link } from "../types";
import { copyText } from "../utils/clipboard";
import { errorMessage } from "../utils/error";
import { filterLinks } from "../utils/filter";
import { pathInBinding } from "../utils/path";
import { ConfirmDialog } from "./ConfirmDialog";
import { LinkRow } from "./LinkRow";

const EMPTY_LINKS: Link[] = [];
const ROW_HEIGHT = 36;
const OPEN_CONFIRM_THRESHOLD = 10;

interface LinkListProps {
  onToast: (message: string) => void;
}

export function LinkList({ onToast }: LinkListProps) {
  const links = useStore((state) => state.snapshot?.links) ?? EMPTY_LINKS;
  const view = useStore((state) => state.view);
  const selectedIds = useStore((state) => state.selectedIds);
  const query = useStore((state) => state.query);
  const searching = query.trim().length > 0;
  const bindings = useStore((state) => state.snapshot?.bindings);
  const bindingPath =
    !searching && view.type === "binding"
      ? bindings?.find((binding) => binding.id === view.id)?.path
      : undefined;
  const visible = useMemo(
    () => filterLinks(links, view, query),
    [links, view, query],
  );
  const visibleIds = useMemo(() => visible.map((link) => link.id), [visible]);
  const listFocusRequest = useStore((state) => state.listFocusRequest);
  const handledFocusRequestRef = useRef(0);
  const anchorIdRef = useRef<string | null>(null);
  const cursorIdRef = useRef<string | null>(null);
  const listRef = useRef<HTMLElement>(null);
  const bodyRef = useRef<HTMLDivElement>(null);
  const [confirming, setConfirming] = useState(false);
  const [confirmingOpen, setConfirmingOpen] = useState(false);
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
  }, [view, query]);

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
    if (cursorIdRef.current && !visibleSet.has(cursorIdRef.current)) {
      cursorIdRef.current = anchorIdRef.current;
    }
  }, [visibleIds]);

  useEffect(() => {
    if (listFocusRequest === handledFocusRequestRef.current) {
      return;
    }
    handledFocusRequestRef.current = listFocusRequest;
    listRef.current?.focus();
    const first = visibleIds[0];
    const current = useStore.getState().selectedIds;
    if (!first || visibleIds.some((id) => current.has(id))) {
      return;
    }
    anchorIdRef.current = first;
    cursorIdRef.current = first;
    useStore.setState({ selectedIds: new Set([first]) });
    bodyRef.current?.scrollTo(0, 0);
  }, [listFocusRequest, visibleIds]);

  const selectedVisibleIds = visibleIds.filter((id) => selectedIds.has(id));

  function applySelect(id: string, mode: SelectMode) {
    const current = {
      selectedIds: useStore.getState().selectedIds,
      anchorId: anchorIdRef.current,
    };
    const next = applySelection(current, visibleIds, id, mode);
    anchorIdRef.current = next.anchorId;
    cursorIdRef.current = id;
    useStore.setState({ selectedIds: next.selectedIds });
    listRef.current?.focus();
  }

  function moveCursor(delta: number, extend: boolean) {
    const cursor = cursorIdRef.current;
    const index = nextCursorIndex(
      visibleIds.length,
      cursor ? visibleIds.indexOf(cursor) : -1,
      delta,
    );
    const id = visibleIds[index];
    if (!id) {
      return;
    }
    applySelect(id, extend ? "range" : "replace");
    virtualizer.scrollToIndex(index, { align: "auto" });
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
    setConfirmingOpen(false);
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

  async function handleCopySelected() {
    const paths = visible
      .filter((link) => selectedIds.has(link.id))
      .map((link) => link.path);
    if (paths.length === 0) {
      return;
    }
    try {
      await copyText(paths.join("\r\n"));
      onToast(
        paths.length === 1 ? "已复制路径" : `已复制 ${paths.length} 个路径`,
      );
    } catch {
      onToast("复制路径失败，请重试或手动选择路径复制");
    }
  }

  async function handleToggleStarSelected() {
    const targets = visible.filter((link) => selectedIds.has(link.id));
    if (targets.length === 0) {
      return;
    }
    const starred = targets.some((link) => !link.starred);
    try {
      for (const link of targets) {
        if (link.starred !== starred) {
          await setStarred(link.id, starred);
        }
      }
    } catch (err) {
      onToast(errorMessage(err, "更新星标失败，请稍后重试"));
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
    if (confirming || confirmingOpen) {
      return;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      moveCursor(event.key === "ArrowDown" ? 1 : -1, event.shiftKey);
      return;
    }
    if (
      (event.ctrlKey || event.metaKey) &&
      event.key.toLowerCase() === "a"
    ) {
      event.preventDefault();
      useStore.setState({ selectedIds: new Set(visibleIds) });
      return;
    }
    if (
      (event.ctrlKey || event.metaKey) &&
      event.key.toLowerCase() === "c"
    ) {
      event.preventDefault();
      void handleCopySelected();
      return;
    }
    if (
      (event.ctrlKey || event.metaKey) &&
      event.key.toLowerCase() === "d"
    ) {
      event.preventDefault();
      void handleToggleStarSelected();
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      if (selectedVisibleIds.length > OPEN_CONFIRM_THRESHOLD) {
        setConfirmingOpen(true);
      } else {
        void handleOpenSelected();
      }
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
        <div aria-hidden="true" />
        <div>名称</div>
        <div>路径</div>
        <div>修改时间</div>
      </div>
      <div
        className="link-body"
        ref={bodyRef}
        style={{ overflowAnchor: "none" }}
      >
        {visible.length === 0 ? (
          <div className="link-empty">
            {view.type === "none"
              ? "请先绑定文件夹"
              : searching
                ? "没有匹配的链接"
                : "暂无链接"}
          </div>
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
                    pathLabel={
                      bindingPath
                        ? pathInBinding(link.path, bindingPath)
                        : link.path
                    }
                    query={query}
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
      <ConfirmDialog
        open={confirmingOpen}
        message={`确定要打开选中的 ${selectedVisibleIds.length} 个文件吗？`}
        confirmLabel="打开"
        onConfirm={() => void handleOpenSelected()}
        onCancel={() => setConfirmingOpen(false)}
      />
    </section>
  );
}
