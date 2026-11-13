import type { ReconcileSummary } from "../types";

export function scanSummaryMessage(summary: ReconcileSummary): string {
  const parts = [
    ["新增", summary.added],
    ["恢复", summary.restored],
    ["移除", summary.removed],
  ]
    .filter(([, count]) => count !== 0)
    .map(([label, count]) => `${label} ${count}`);
  return parts.length === 0
    ? "扫描完成，没有变化"
    : `扫描完成：${parts.join("，")}`;
}
