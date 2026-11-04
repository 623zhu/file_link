import type { Link, View } from "../types";

export function hasManualSource(link: Link): boolean {
  return link.sources.some((source) => source.type === "manual");
}

export function hasBindingSource(link: Link, bindingId: string): boolean {
  return link.sources.some(
    (source) => source.type === "binding" && source.bindingId === bindingId,
  );
}

export function filterLinks(links: Link[], view: View, query = ""): Link[] {
  const viewed = filterByView(links, view);
  const matched = filterByQuery(viewed, query);
  return sortLinks(matched);
}

function filterByView(links: Link[], view: View): Link[] {
  if (view.type === "manual") {
    return links.filter(hasManualSource);
  }
  if (view.type === "binding") {
    return links.filter((link) => hasBindingSource(link, view.id));
  }
  return links;
}

function filterByQuery(links: Link[], query: string): Link[] {
  const needle = query.trim().toLowerCase();
  if (needle.length === 0) {
    return links;
  }
  return links.filter(
    (link) =>
      link.name.toLowerCase().includes(needle) ||
      link.path.toLowerCase().includes(needle),
  );
}

function sortLinks(links: Link[]): Link[] {
  return [...links].sort((a, b) => {
    if (a.missing !== b.missing) {
      return a.missing ? 1 : -1;
    }
    const byName = a.name.localeCompare(b.name, "en", { sensitivity: "base" });
    if (byName !== 0) {
      return byName;
    }
    return a.path.localeCompare(b.path, "en", { sensitivity: "base" });
  });
}
