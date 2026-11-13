import { describe, expect, it } from "vitest";

import { scanSummaryMessage } from "./scan";

describe("scanSummaryMessage", () => {
  it("says nothing changed when every count is zero", () => {
    expect(scanSummaryMessage({ added: 0, restored: 0, removed: 0 })).toBe(
      "扫描完成，没有变化",
    );
  });

  it("lists only the counts that changed", () => {
    expect(scanSummaryMessage({ added: 3, restored: 0, removed: 0 })).toBe(
      "扫描完成：新增 3",
    );
    expect(scanSummaryMessage({ added: 2, restored: 0, removed: 1 })).toBe(
      "扫描完成：新增 2，移除 1",
    );
    expect(scanSummaryMessage({ added: 1, restored: 4, removed: 5 })).toBe(
      "扫描完成：新增 1，恢复 4，移除 5",
    );
  });
});
