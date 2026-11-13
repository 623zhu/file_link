import { describe, expect, it } from "vitest";

import { splitHighlight } from "./highlight";

describe("splitHighlight", () => {
  it("returns the whole text unmarked without a query", () => {
    expect(splitHighlight("Report.html", "")).toEqual([
      { text: "Report.html", hit: false },
    ]);
    expect(splitHighlight("Report.html", "   ")).toEqual([
      { text: "Report.html", hit: false },
    ]);
  });

  it("returns the whole text unmarked when nothing matches", () => {
    expect(splitHighlight("Report.html", "pdf")).toEqual([
      { text: "Report.html", hit: false },
    ]);
  });

  it("marks every match ignoring case and keeps the original casing", () => {
    expect(splitHighlight("D:\\Docs\\docs.html", "DOCS")).toEqual([
      { text: "D:\\", hit: false },
      { text: "Docs", hit: true },
      { text: "\\", hit: false },
      { text: "docs", hit: true },
      { text: ".html", hit: false },
    ]);
  });

  it("trims the query and handles matches at both ends", () => {
    expect(splitHighlight("abcab", " ab ")).toEqual([
      { text: "ab", hit: true },
      { text: "c", hit: false },
      { text: "ab", hit: true },
    ]);
  });

  it("does not overlap matches", () => {
    expect(splitHighlight("aaa", "aa")).toEqual([
      { text: "aa", hit: true },
      { text: "a", hit: false },
    ]);
  });
});
