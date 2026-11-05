import type { Link, View } from "../types";

export function hasBindingSource(link: Link, bindingId: string): boolean {
  return link.sources.some(
    (source) => source.type === "binding" && source.bindingId === bindingId,
  );
}

export function filterLinks(links: Link[], view: View): Link[] {
  return sortLinks(filterByView(links, view));
}

function filterByView(links: Link[], view: View): Link[] {
  if (view.type === "none") {
    return [];
  }
  return links.filter((link) => hasBindingSource(link, view.id));
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
