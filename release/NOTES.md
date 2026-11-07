# File Link 0.1.0

免安装版：`FileLink_0.1.0_x64_portable.exe`

SHA256：

```
7F5533453CDD4409501E1B11C6B1D5B9626548A7C2D3B2865FA43C029CA96809
```

也可用同目录的 `SHA256SUMS.txt` 校验：

```powershell
Get-FileHash .\FileLink_0.1.0_x64_portable.exe -Algorithm SHA256
```

## 运行要求

- Windows 10 / 11，x64
- 已安装 WebView2 Runtime（Windows 11 自带；Windows 10 大多已随 Edge 安装）。若无法启动，请到微软官网安装「WebView2 Runtime」

拷到任意目录双击即可运行，不需要安装 Node.js 或其它开发环境。链接数据写在 `%APPDATA%\com.filelink.app\`。

`release/` 只保留这一份 exe。以后重新构建会直接覆盖它，并删掉同目录旧包。

## 本版本内容

- 绑定文件夹后按类型规则生成链接
- 自动导入或手动扫描新文件
- 打开、在资源管理器中定位、移除链接（只删除链接，不删除原文件）
- 按文件夹或文件类型浏览、多选批量操作、键盘操作、长列表虚拟滚动
- 绑定文件夹时可配置该目录的文件类型；链接失效时标记「缺失」
- 自定义应用图标；release 构建无控制台窗口
- 不提供全部链接 / 手动导入视图，不提供搜索

## 已知问题

- 可执行文件尚未代码签名。首次运行时，Windows SmartScreen 可能提示「Windows 已保护你的电脑」，可点「更多信息 → 仍要运行」
- 免安装版不能自动补装 WebView2
- 安装程序与代码签名安排在后续版本
