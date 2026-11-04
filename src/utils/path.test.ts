import { describe, expect, it } from "vitest";

import {
  autoImportedMessage,
  folderName,
  isNetworkPath,
  NETWORK_WATCH_HINT,
} from "./path";

describe("isNetworkPath", () => {
  it("treats UNC paths as network paths", () => {
    expect(isNetworkPath("\\\\server\\share\\docs")).toBe(true);
    expect(isNetworkPath("//server/share/docs")).toBe(true);
    expect(isNetworkPath("\\\\?\\UNC\\server\\share")).toBe(true);
  });

  it("does not treat local paths as network paths", () => {
    expect(isNetworkPath("D:\\notes")).toBe(false);
    expect(isNetworkPath("C:\\Users\\L\\docs")).toBe(false);
    expect(isNetworkPath("\\\\?\\C:\\Users\\L\\docs")).toBe(false);
  });
});

describe("folderName", () => {
  it("returns the last path segment", () => {
    expect(folderName("D:\\notes\\tools")).toBe("tools");
    expect(folderName("\\\\server\\share\\report")).toBe("report");
  });
});

describe("autoImportedMessage", () => {
  it("uses the folder name in the toast copy", () => {
    expect(autoImportedMessage("D:\\notes\\tools", 3)).toBe(
      "已从 tools 自动导入 3 个文件",
    );
  });

  it("falls back when the binding is missing", () => {
    expect(autoImportedMessage(undefined, 1)).toBe(
      "已从 文件夹 自动导入 1 个文件",
    );
  });
});

describe("NETWORK_WATCH_HINT", () => {
  it("matches the copy in DEV.md", () => {
    expect(NETWORK_WATCH_HINT).toBe(
      "网络目录的监视可能不可靠，建议配合手动扫描",
    );
  });
});
