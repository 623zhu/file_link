import { describe, expect, it } from "vitest";

import type { Link, Source } from "../types";
import { filterLinks } from "./filter";

function link(id: string, sources: Source[], missing = false): Link {
  return {
    id,
    path: `D:\\files\\${id}.html`,
    key: `d:\\files\\${id}.html`,
    name: `${id}.html`,
    sources,
    addedAt: "2026-11-04T15:00:00+08:00",
    missing,
  };
}

const manual: Source = { type: "manual" };
const binding: Source = { type: "binding", bindingId: "b1" };

const links: Link[] = [
  link("only-manual", [manual]),
  link("only-binding", [binding]),
  link("both", [manual, binding]),
  link("missing-manual", [manual], true),
];

describe("filterLinks", () => {
  it("returns every link for the all view", () => {
    expect(filterLinks(links, { type: "all" })).toEqual(links);
  });

  it("keeps only links with a manual source for the manual view", () => {
    expect(filterLinks(links, { type: "manual" })).toEqual([
      links[0],
      links[2],
      links[3],
    ]);
  });
});
