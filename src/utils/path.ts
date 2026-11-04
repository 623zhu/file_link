export const NETWORK_WATCH_HINT = "网络目录的监视可能不可靠，建议配合手动扫描";

export function isNetworkPath(path: string): boolean {
  const unified = path.trim().replace(/\//g, "\\");
  if (!unified.startsWith("\\\\")) {
    return false;
  }
  const rest = unified.slice(2).toLowerCase();
  if (rest.startsWith("?\\unc\\")) {
    return true;
  }
  return !rest.startsWith("?\\");
}

export function folderName(path: string): string {
  const parts = path.split(/[\\/]/).filter((part) => part.length > 0);
  return parts[parts.length - 1] ?? path;
}

export function autoImportedMessage(
  bindingPath: string | undefined,
  added: number,
): string {
  const name = bindingPath ? folderName(bindingPath) : "文件夹";
  return `已从 ${name} 自动导入 ${added} 个文件`;
}
