import { useMemo } from "react";

import { filterLinks } from "../utils/filter";
import { useStore } from "../store";
import type { Binding, Link } from "../types";
import { LinkRow } from "./LinkRow";

const EMPTY_LINKS: Link[] = [];
const EMPTY_BINDINGS: Binding[] = [];

export function LinkList() {
  const links = useStore((state) => state.snapshot?.links) ?? EMPTY_LINKS;
  const bindings =
    useStore((state) => state.snapshot?.bindings) ?? EMPTY_BINDINGS;
  const view = useStore((state) => state.view);
  const visible = useMemo(() => filterLinks(links, view), [links, view]);

  return (
    <section className="link-list" aria-label="链接列表">
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
            <LinkRow key={link.id} link={link} bindings={bindings} />
          ))
        )}
      </div>
    </section>
  );
}
