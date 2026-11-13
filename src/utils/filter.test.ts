import { describe, expect, it } from "vitest";

import type { Link, Source } from "../types";
import { filterLinks, listedExtensions } from "./filter";

function link(
  id: string,
  sources: Source[],
  missing = false,
  extras: Partial<Pick<Link, "name" | "path" | "starred">> = {},
): Link {
  return {
    id,
    path: extras.path ?? `D:\\files\\${id}.html`,
    key: `d:\\files\\${id}.html`,
    name: extras.name ?? `${id}.html`,
    sources,
    modifiedAt: "2026-11-04T15:00:00+08:00",
    missing,
    starred: extras.starred ?? false,
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

describe("filterLinks search", () => {
  const view = { type: "binding", id: "b1" } as const;
  const searchable: Link[] = [
    link("report", [binding], false, {
      name: "Report.html",
      path: "D:\\work\\Report.html",
    }),
    link("notes", [binding], false, {
      name: "notes.html",
      path: "D:\\Projects\\notes.html",
    }),
    link("star", [binding], false, {
      name: "z-report-old.html",
      path: "D:\\work\\z-report-old.html",
      starred: true,
    }),
    link("elsewhere", [otherBinding], false, {
      name: "report-b2.html",
      path: "D:\\b2\\report-b2.html",
    }),
    link("manual-report", [manual], false, {
      name: "report-manual.html",
      path: "D:\\manual\\report-manual.html",
    }),
  ];

  it("matches names case-insensitively across all bindings", () => {
    expect(ids(filterLinks(searchable, view, "REPORT"))).toEqual([
      "star",
      "elsewhere",
      "report",
    ]);
  });

  it("ignores the current view and skips manual-only links", () => {
    const other = { type: "extension", ext: ".md" } as const;
    expect(ids(filterLinks(searchable, other, "report"))).not.toContain(
      "manual-report",
    );
    expect(ids(filterLinks(searchable, other, "b2"))).toEqual(["elsewhere"]);
  });

  it("matches paths", () => {
    expect(ids(filterLinks(searchable, view, "projects\\"))).toEqual(["notes"]);
  });

  it("trims the query and treats blank as no filter", () => {
    expect(ids(filterLinks(searchable, view, "  notes  "))).toEqual(["notes"]);
    expect(ids(filterLinks(searchable, view, "   "))).toEqual([
      "star",
      "notes",
      "report",
    ]);
  });

  it("returns nothing when no link matches", () => {
    expect(filterLinks(searchable, view, "missing")).toEqual([]);
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

  it("pins starred links to the top", () => {
    const unsorted: Link[] = [
      link("zeta", [binding]),
      link("alpha", [binding]),
      link("Beta", [binding], false, { starred: true }),
      link("gamma", [binding], false, { starred: true }),
    ];
    expect(ids(filterLinks(unsorted, { type: "binding", id: "b1" }))).toEqual([
      "Beta",
      "gamma",
      "alpha",
      "zeta",
    ]);
  });
});
