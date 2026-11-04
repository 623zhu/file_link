import type { AppError } from "../types";

export function errorMessage(err: unknown, fallback: string): string {
  if (err && typeof err === "object" && "code" in err && "message" in err) {
    const message = (err as AppError).message;
    if (typeof message === "string" && message.length > 0) {
      return message;
    }
  }
  return fallback;
}

export function dataRecoveredMessage(backupPath: string): string {
  return `数据文件已损坏，已备份到 ${backupPath}，当前以空数据启动。`;
}
