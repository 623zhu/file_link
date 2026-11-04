import { describe, expect, it } from "vitest";

import type { Link, Source } from "../types";
import { filterLinks } from "./filter";

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
    addedAt: "2026-11-04T15:00:00+08:00",
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
  link("missing-manual", [manual], true),
  link("other-binding", [otherBinding]),
];

function ids(result: Link[]): string[] {
  return result.map((item) => item.id);
}

describe("filterLinks", () => {
  it("returns every link for the all view, sorted", () => {
    expect(ids(filterLinks(links, { type: "all" }))).toEqual([
      "both",
      "only-binding",
      "only-manual",
      "other-binding",
      "missing-manual",
    ]);
  });

  it("keeps only links with a manual source for the manual view", () => {
    expect(ids(filterLinks(links, { type: "manual" }))).toEqual([
      "both",
      "only-manual",
      "missing-manual",
    ]);
  });

  it("keeps links that include the selected binding", () => {
    expect(ids(filterLinks(links, { type: "binding", id: "b1" }))).toEqual([
      "both",
      "only-binding",
    ]);
  });

  it("excludes links that only belong to another binding", () => {
    expect(ids(filterLinks(links, { type: "binding", id: "b2" }))).toEqual([
      "other-binding",
    ]);
  });
});

describe("filterLinks search", () => {
  it("matches name case-insensitively", () => {
    expect(ids(filterLinks(links, { type: "all" }, "BOTH"))).toEqual(["both"]);
  });

  it("matches path case-insensitively", () => {
    const mixed: Link[] = [
      link("alpha", [manual], false, {
        name: "alpha.html",
        path: "D:\\notes\\alpha.html",
      }),
      link("report", [manual], false, {
        name: "q1.html",
        path: "D:\\Reports\\q1.html",
      }),
    ];
    expect(ids(filterLinks(mixed, { type: "all" }, "reports"))).toEqual([
      "report",
    ]);
  });

  it("treats blank query as no search filter", () => {
    expect(ids(filterLinks(links, { type: "all" }, "   "))).toEqual(
      ids(filterLinks(links, { type: "all" })),
    );
  });

  it("applies the current view before searching", () => {
    expect(ids(filterLinks(links, { type: "manual" }, "binding"))).toEqual([]);
    expect(ids(filterLinks(links, { type: "all" }, "binding"))).toEqual([
      "only-binding",
      "other-binding",
    ]);
  });
});

describe("filterLinks sort", () => {
  it("puts missing links last and sorts the rest by name", () => {
    const unsorted: Link[] = [
      link("zeta", [manual]),
      link("alpha", [manual], true),
      link("Beta", [manual]),
      link("gamma", [manual], true),
    ];
    expect(ids(filterLinks(unsorted, { type: "all" }))).toEqual([
      "Beta",
      "zeta",
      "alpha",
      "gamma",
    ]);
  });
});
