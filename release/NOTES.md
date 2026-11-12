# File Link 0.1.0

免安装版：`FileLink_0.1.0_x64_portable.exe`  
安装程序：`FileLink_0.1.0_x64-setup.exe`（当前用户安装、简体中文）

SHA256：

```
17F77BBED031F78B88A867D566BEB61E9DD78E620FB4BFD3470981E42B1734C8  FileLink_0.1.0_x64_portable.exe
C8AB1389BCEB0F7C94F7098630C15B82F7A2621F1A900F87D92F3A27DA99DC05  FileLink_0.1.0_x64-setup.exe
```

也可用同目录的 `SHA256SUMS.txt` 校验：

```powershell
Get-FileHash .\FileLink_0.1.0_x64_portable.exe -Algorithm SHA256
Get-FileHash .\FileLink_0.1.0_x64-setup.exe -Algorithm SHA256
```

## 运行要求

- Windows 10 / 11，x64
- 免安装版需要已安装 WebView2 Runtime（Windows 11 自带；Windows 10 大多已随 Edge 安装）。若无法启动，请到微软官网安装「WebView2 Runtime」，或改用安装程序。
- 安装程序会在安装时检测 WebView2，缺失则联网补装。

免安装版拷到任意目录双击即可运行。安装程序安装到 `%LOCALAPPDATA%\File Link\`，不需要管理员权限。两种形式的链接数据都写在 `%APPDATA%\com.filelink.app\`。

`release/` 只保留最新一份免安装版和最新一份安装程序。以后重新构建会直接覆盖它们，并删掉同目录旧包。

## 本版本内容

- 绑定文件夹后按类型规则生成链接
- 自动导入或手动扫描新文件
- 打开、在资源管理器中定位、移除链接（只删除链接，不删除原文件）
- 按文件夹或文件类型浏览、多选批量操作、键盘操作、长列表虚拟滚动
- 绑定文件夹时可配置该目录的文件类型；原文件删除或移走后从列表移除链接
- 链接可标星，已标星的条目在当前列表置顶
- 自定义应用图标；release 构建无控制台窗口
- NSIS 安装程序：当前用户安装、简体中文，卸载时默认保留链接数据
- 不提供全部链接 / 手动导入视图，不提供搜索

## 已知问题

- 可执行文件尚未代码签名。首次运行时，Windows SmartScreen 可能提示「Windows 已保护你的电脑」，可点「更多信息 → 仍要运行」
- 代码签名安排在后续版本
