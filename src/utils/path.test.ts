import { describe, expect, it } from "vitest";

import {
  autoImportedMessage,
  folderName,
  isNetworkPath,
  NETWORK_WATCH_HINT,
  pathInBinding,
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

describe("pathInBinding", () => {
  const binding = "D:\\work\\tools";

  it("shows the binding name with the sub folder", () => {
    expect(pathInBinding("D:\\work\\tools\\docs\\api\\a.html", binding)).toBe(
      "tools\\docs\\api",
    );
  });

  it("shows only the binding name for files at its root", () => {
    expect(pathInBinding("D:\\work\\tools\\a.html", binding)).toBe("tools");
  });

  it("matches the prefix case-insensitively", () => {
    expect(pathInBinding("d:\\WORK\\Tools\\docs\\a.html", binding)).toBe(
      "tools\\docs",
    );
  });

  it("handles a drive root binding", () => {
    expect(pathInBinding("C:\\a.html", "C:\\")).toBe("C:");
    expect(pathInBinding("C:\\docs\\a.html", "C:\\")).toBe("C:\\docs");
  });

  it("keeps the full path outside the binding", () => {
    expect(pathInBinding("D:\\work\\toolsx\\a.html", binding)).toBe(
      "D:\\work\\toolsx\\a.html",
    );
    expect(pathInBinding("E:\\other\\a.html", binding)).toBe(
      "E:\\other\\a.html",
    );
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
