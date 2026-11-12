import { describe, expect, it } from "vitest";

import { dataRecoveredMessage, errorMessage } from "./error";

describe("errorMessage", () => {
  it("shows the backend message for AppError", () => {
    expect(
      errorMessage(
        {
          code: "FileMissing",
          message: "文件不存在或已被移动，已从列表中移除该链接",
        },
        "打开失败，请确认文件仍在原位置",
      ),
    ).toBe("文件不存在或已被移动，已从列表中移除该链接");
  });

  it("ignores english plugin errors and uses the fallback", () => {
    expect(
      errorMessage(
        { message: "Access is denied. (os error 5)" },
        "无法在资源管理器中打开该文件夹，请确认目录仍存在",
      ),
    ).toBe("无法在资源管理器中打开该文件夹，请确认目录仍存在");
  });
});

describe("dataRecoveredMessage", () => {
  it("includes the backup path", () => {
    expect(
      dataRecoveredMessage(
        "C:\\Users\\L\\AppData\\Roaming\\com.filelink.app\\data.json.broken-1",
      ),
    ).toBe(
      "数据文件已损坏，已备份到 C:\\Users\\L\\AppData\\Roaming\\com.filelink.app\\data.json.broken-1，当前以空数据启动。",
    );
  });
});
