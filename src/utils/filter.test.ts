import { describe, expect, it } from "vitest";

import type { Link, Source } from "../types";
import { filterLinks, listedExtensions } from "./filter";

function link(
  id: string,
  sources: Source[],
  missing = false,
  extras: Partial<Pick<Link, "name" | "path">> = {},
): Link {
  return {
    id,
    path: extras.path ?? `D:\\files\\${id}.html`,
    key: `d:\\files\\${id}.html`,
    name: extras.name ?? `${id}.html`,
    sources,
    modifiedAt: "2026-11-04T15:00:00+08:00",
    missing,
  };
}

const manual: Source = { type: "manual" };
const binding: Source = { type: "binding", bindingId: "b1" };
const otherBinding: Source = { type: "binding", bindingId: "b2" };

const links: Link[] = [
  link("only-manual", [manual]),
  link("only-binding", [binding]),
  link("both", [manual, binding]),
  link("missing-binding", [binding], true),
  link("other-binding", [otherBinding]),
  link("note", [binding], false, {
    name: "note.md",
    path: "D:\\files\\note.md",
  }),
];

function ids(result: Link[]): string[] {
  return result.map((item) => item.id);
}

describe("filterLinks", () => {
  it("returns no links when no binding is selected", () => {
    expect(ids(filterLinks(links, { type: "none" }))).toEqual([]);
  });

  it("keeps links that include the selected binding", () => {
    expect(ids(filterLinks(links, { type: "binding", id: "b1" }))).toEqual([
      "both",
      "missing-binding",
      "note",
      "only-binding",
    ]);
  });

  it("excludes links that only belong to another binding", () => {
    expect(ids(filterLinks(links, { type: "binding", id: "b2" }))).toEqual([
      "other-binding",
    ]);
  });

  it("hides links that only have a manual source", () => {
    expect(ids(filterLinks(links, { type: "binding", id: "b1" }))).not.toContain(
      "only-manual",
    );
  });

  it("keeps bound links of the selected file type", () => {
    expect(ids(filterLinks(links, { type: "extension", ext: ".html" }))).toEqual(
      ["both", "missing-binding", "only-binding", "other-binding"],
    );
    expect(ids(filterLinks(links, { type: "extension", ext: ".md" }))).toEqual([
      "note",
    ]);
  });

  it("hides manual-only links from file type views", () => {
    expect(
      ids(filterLinks(links, { type: "extension", ext: ".html" })),
    ).not.toContain("only-manual");
  });
});

describe("listedExtensions", () => {
  it("lists bound extensions and ignores manual-only links", () => {
    expect(listedExtensions(links)).toEqual([".html", ".md"]);
  });
});

describe("filterLinks sort", () => {
  it("sorts by name", () => {
    const unsorted: Link[] = [
      link("zeta", [binding]),
      link("alpha", [binding]),
      link("Beta", [binding]),
      link("gamma", [binding]),
    ];
    expect(ids(filterLinks(unsorted, { type: "binding", id: "b1" }))).toEqual([
      "alpha",
      "Beta",
      "gamma",
      "zeta",
    ]);
  });
});
