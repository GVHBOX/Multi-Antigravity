<div align="center">

<img src="assets/icon.png" alt="Multi-Antigravity Logo" width="96" height="96" />

# Multi-Antigravity

**Google Antigravity 双开与多实例管理工具**

让主账号与分身账号在同一台电脑上同时运行，互不干扰。

[![CI](https://github.com/GVHBOX/multi-antigravity/actions/workflows/ci.yml/badge.svg)](https://github.com/GVHBOX/multi-antigravity/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%2F11-0078D6?logo=windows&logoColor=white)](https://github.com/GVHBOX/multi-antigravity)
[![Rust](https://img.shields.io/badge/Rust-2024%20Edition-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![UI](https://img.shields.io/badge/UI-Slint%20Native-00599C)](https://slint.dev/)

<br/>

<img src="assets/preview.png" alt="Multi-Antigravity 界面预览" width="100%" />

</div>

---

## 它是干什么用的？

官方 Google Antigravity 只能登录一个账号，切换账号需要反复登出重新登录。

使用本工具可以在电脑上**同时打开第二个（或更多）Antigravity 窗口**，登录不同的 Google 账号：
- **主账号**：正常使用，原有配置和登录状态保持不变。
- **分身账号**：配置和登录凭据保存在本地独立的 `data/` 目录中。
- **互不干扰**：主号与分身各自独立运行，聊天记录、扩展插件、本地数据完全隔离。

---

## 3 步开始使用

1. **打开软件**  
   双击运行 `Multi-Antigravity.exe`。
2. **启动分身**  
   点击界面上的「启动」按钮（或按键盘空格键 <kbd>Space</kbd>）。
3. **登录账号**  
   在弹出的 Antigravity 窗口中登录你的第二个 Google 账号。

后续需要使用时，直接打开软件点「启动」即可自动进入分身。

---

## 常用操作

| 操作 | 方式 | 说明 |
| :--- | :--- | :--- |
| **启动 / 停止分身** | 点界面「启动 / 停止」按钮，或按 <kbd>Space</kbd> | 打开或关闭分身窗口 |
| **更换分身账号** | 点界面「登出」按钮，或按 <kbd>C</kbd> | 清除分身登录凭据，下次启动会重新弹出登录页面 |
| **打开数据文件夹** | 点界面「目录」按钮，或按 <kbd>O</kbd> | 打开分身本地存储目录（`data/instance_2`） |
| **重启分身后台** | 按快捷键 <kbd>R</kbd> | 重新加载分身后台服务 |
| **开关本地代理** | 点界面「7890 代理」按钮，或按 <kbd>P</kbd> | 开启或关闭本地 7890 端口代理支持 |
| **切换扩展槽位** | 点界面右上角「双开 / 四开」开关 | 支持同时管理多个分身实例 |
| **最小化到托盘** | 点击窗口右上角关闭按钮 `X` | 软件自动最小化到系统右下角托盘，右键托盘图标可退出 |

---

## 常见问题（FAQ）

**Q：需要先退出正在运行的主号吗？**  
不需要。主号与分身彼此完全独立，主号正在使用时可直接启动分身。

**Q：分身的数据保存在哪里？**  
保存在程序所在目录的 `data/` 文件夹下。如果你想备份、迁移或者删除分身数据，直接操作该文件夹即可。

**Q：会泄露我的账号凭据或上传隐私数据吗？**  
不会。分身直接连接 Google 官方服务器，本工具不架设网络中转，所有登录凭据仅保存在你本机的 `data/` 目录中，不收集也不上传任何数据。

**Q：点击「启动」后没有弹出窗口？**  
请确认你的电脑上已经正常安装了官方 Google Antigravity 客户端（默认安装路径为系统 `AppData\Local\Programs\antigravity`）。

---

## 键盘快捷键速查

| 按键 | 功能 |
| :---: | :--- |
| <kbd>Space</kbd> | 启动 / 停止分身 |
| <kbd>K</kbd> | 停止分身进程 |
| <kbd>R</kbd> | 重启分身服务 |
| <kbd>C</kbd> | 清除登录凭据（登出） |
| <kbd>O</kbd> | 打开分身数据目录 |
| <kbd>L</kbd> | 切换运行日志与服务日志 |
| <kbd>P</kbd> | 开关本地 7890 代理 |

---

## 高级用法（可选）

### 终端字符界面（TUI）
在命令行运行：
```powershell
.\Multi-Antigravity.exe --tui
```

### 从源码编译
需要 Rust 1.85+ 及 MSVC 构建环境：
```powershell
cargo build --release
```
编译产物位于 `target\release\multi-antigravity-rust.exe`，复制至根目录重命名为 `Multi-Antigravity.exe` 即可。

---

## 开源协议

本项目采用 [MIT License](LICENSE) 开源。
