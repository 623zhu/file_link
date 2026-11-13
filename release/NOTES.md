# File Link 0.2.0

免安装版：`FileLink_0.2.0_x64_portable.exe`  
安装程序：`FileLink_0.2.0_x64-setup.exe`（当前用户安装、简体中文）

SHA256：

```
66C0038F0C80F8588AF2C54136EF006A9C30059ABDE966EDDFD019FED9A98FFC  FileLink_0.2.0_x64_portable.exe
4C09915DAE14BC2C65D6FB22B74F689380C6AD42F8F7F3C18911C0B6306D6C8C  FileLink_0.2.0_x64-setup.exe
```

也可用同目录的 `SHA256SUMS.txt` 校验：

```powershell
Get-FileHash .\FileLink_0.2.0_x64_portable.exe -Algorithm SHA256
Get-FileHash .\FileLink_0.2.0_x64-setup.exe -Algorithm SHA256
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
- 按文件夹或文件类型浏览、多选批量操作、长列表虚拟滚动
- 绑定文件夹时可配置该目录的文件类型；原文件删除或移走后从列表移除链接
- 链接可标星，已标星的条目在当前列表置顶；Ctrl+D 切换选中项星标
- 按文件名或路径搜索全部绑定文件夹，命中文字高亮
- 键盘：Ctrl+F 搜索、方向键选择、Ctrl+A 全选、Ctrl+C 复制路径
- 一次打开超过 10 个文件前先确认
- 启动时恢复上次查看的文件夹或类型，以及窗口大小和位置
- 文件夹视图的路径列只显示相对子目录
- 绑定文件夹内改名或移动后沿用原链接，并保留星标
- 自定义应用图标；release 构建无控制台窗口
- NSIS 安装程序：当前用户安装、简体中文，卸载时默认保留链接数据
- 不提供全部链接 / 手动导入视图

## 已知问题

- 可执行文件尚未代码签名。首次运行时，Windows SmartScreen 可能提示「Windows 已保护你的电脑」，可点「更多信息 → 仍要运行」
- 代码签名安排在后续版本
