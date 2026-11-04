import type { Link, View } from "../types";

export function hasManualSource(link: Link): boolean {
  return link.sources.some((source) => source.type === "manual");
}

export function filterLinks(links: Link[], view: View): Link[] {
  if (view.type === "manual") {
    return links.filter(hasManualSource);
  }
  return links;
}
