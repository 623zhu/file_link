import type { Link, View } from "../types";

export function hasManualSource(link: Link): boolean {
  return link.sources.some((source) => source.type === "manual");
}

export function hasBindingSource(link: Link, bindingId: string): boolean {
  return link.sources.some(
    (source) => source.type === "binding" && source.bindingId === bindingId,
  );
}

export function filterLinks(links: Link[], view: View): Link[] {
  if (view.type === "manual") {
    return links.filter(hasManualSource);
  }
  if (view.type === "binding") {
    return links.filter((link) => hasBindingSource(link, view.id));
  }
  return links;
}
