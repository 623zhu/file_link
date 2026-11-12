import type { Link, View } from "../types";

export function hasBindingSource(link: Link, bindingId: string): boolean {
  return link.sources.some(
    (source) => source.type === "binding" && source.bindingId === bindingId,
  );
}

export function hasAnyBindingSource(link: Link): boolean {
  return link.sources.some((source) => source.type === "binding");
}

export function linkExtension(link: Link): string {
  const fromName = extensionOf(link.name);
  if (fromName.length > 0) {
    return fromName;
  }
  return extensionOf(link.path);
}

export function listedExtensions(links: readonly Link[]): string[] {
  const found = new Set<string>();
  for (const link of links) {
    if (!hasAnyBindingSource(link)) {
      continue;
    }
    const ext = linkExtension(link);
    if (ext.length > 0) {
      found.add(ext);
    }
  }
  return [...found].sort();
}

export function extensionLabel(ext: string): string {
  return ext.replace(/^\./, "").toUpperCase();
}

export function filterLinks(links: Link[], view: View): Link[] {
  return sortLinks(filterByView(links, view));
}

function filterByView(links: Link[], view: View): Link[] {
  if (view.type === "none") {
    return [];
  }
  if (view.type === "binding") {
    return links.filter((link) => hasBindingSource(link, view.id));
  }
  return links.filter(
    (link) => hasAnyBindingSource(link) && linkExtension(link) === view.ext,
  );
}

function extensionOf(value: string): string {
  const slash = Math.max(value.lastIndexOf("\\"), value.lastIndexOf("/"));
  const name = slash >= 0 ? value.slice(slash + 1) : value;
  const dot = name.lastIndexOf(".");
  if (dot <= 0 || dot === name.length - 1) {
    return "";
  }
  return name.slice(dot).toLowerCase();
}

function sortLinks(links: Link[]): Link[] {
  return [...links].sort((a, b) => {
    const byName = a.name.localeCompare(b.name, "en", { sensitivity: "base" });
    if (byName !== 0) {
      return byName;
    }
    return a.path.localeCompare(b.path, "en", { sensitivity: "base" });
  });
}
