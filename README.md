# Antigravity 多实例管理器 (Antigravity Multi-Instance Manager) v1.0.0

用 Rust 构建的 Google Antigravity 多开调度、环境隔离与进程监控工具。支持桌面 GUI 客户端与终端 TUI 双模运行。

---

## 核心特性

- **轻量桌面客户端（默认 Slint 运行）**：
  - 基于 Slint 矢量引擎（`winit-software` 软件光栅后端，不依赖 GPU），无 Chromium / WebView2 运行时依赖。
  - 单进程内存开销约 15 MB，无附加子进程，启动迅速。
  - 零运行缓存污染：纯代码自绘，不生成 `.WebView2` 或额外浏览器缓存。
  - 点击关闭缩小至系统托盘常驻，单击/双击托盘图标唤回座舱，支持右键托盘快捷控制。
  - 资源离线自包含：无外部 CDN 或网络字体依赖。

- **终端 TUI 模式（`--tui` 参数）**：
  - 基于 Ratatui 构建的单屏紧凑仪表盘，24 行内显示完整状态。
  - 实时采集双实例 CPU、物理内存 (RSS) 及语言服务端口。

- **后台独立子进程 (Detached)**：
  - 通过 `CREATE_BREAKAWAY_FROM_JOB` 与 `DETACHED_PROCESS` 派生子进程。
  - 关闭座舱不影响已运行的分身，随时退出、随时重连接管。

- **多账号环境隔离**：
  - `USERPROFILE` 与 `HOME` 重定向至 `data/instance_2/home`，避免配置文件冲突。
  - `APPDATA` 重定向至 `data/instance_2/AppData/Roaming`，独立扩展与缓存。
  - 注入 `SSH_CONNECTION` 环境变量旁路，强制语言服务使用独立文件凭据 (`jetski-standalone-oauth-token`)。
  - 保持 `LOCALAPPDATA` 系统默认路径，直接复用已有的 Chrome 登录环境。

---

## 快速启动

### 1. 桌面 GUI 客户端（推荐）
直接运行根目录下的可执行文件或批处理脚本：
```text
antigravity-cockpit.exe
或
start.bat
```

### 2. 终端 TUI 仪表盘
在命令行或终端中以字符模式运行：
```powershell
.\antigravity-cockpit.exe --tui
或
双击运行 start_tui.bat
```

### 3. 从源码编译
```powershell
cargo build --release
```
产物位于 `target/release/multi-antigravity-rust.exe`。
根目录的 `antigravity-cockpit.exe` 为该产物的发布副本：

```powershell
copy target\release\multi-antigravity-rust.exe antigravity-cockpit.exe
```

---

## 快捷键操作

| 按键 | 功能 | 说明 | 适用 |
| :---: | :--- | :--- | :---: |
| **<kbd>Space</kbd>** | **启动 / 唤醒分身** | 未运行时启动分身；运行中时唤醒分身窗口置顶 | GUI / TUI |
| **<kbd>K</kbd>** | **停止分身** | 终止分身进程树 | GUI / TUI |
| **<kbd>R</kbd>** | **重启语言服务** | 终止分身语言服务进程，触发自动重启刷新端口 | GUI / TUI |
| **<kbd>C</kbd>** | **清除独立凭据** | 删除独立 Token 文件，下次打开时可重新登录账号 | GUI / TUI |
| **<kbd>O</kbd>** | **打开沙箱目录** | 打开 `data/instance_2` 目录 | GUI / TUI |
| **<kbd>T</kbd>** | **中 / EN 切换** | 切换界面语言 | 仅 TUI |
| **<kbd>L</kbd>** | **控制台清屏** | 清空日志缓冲区 | 仅 TUI |
| **<kbd>Q</kbd>** | **退出座舱** | 退出座舱应用（窗口点击 `[✕]` 为最小化至托盘） | GUI / TUI |

---

## 目录结构

```text
multi-antigravity-rust/
├── assets/                 # 静态资源（图标源图、.ico、托盘数据）
│   ├── icon.ico            # Windows PE 资源图标
│   ├── icon.png            # 高清图标母版
│   └── icon_32.rgba        # 托盘像素矩阵
├── docs/                   # 项目文档与视觉设计稿
│   └── cockpit-ui-prototype.html # 1920×1080 满屏界面视觉原型
├── src/                    # Rust 核心源代码
│   ├── app.rs              # 状态机与事件调度
│   ├── gui.rs              # Slint GUI 桌面端与系统托盘
│   ├── i18n.rs             # 多语言文案字典
│   ├── launcher.rs         # 进程派生、凭据管理与环境隔离
│   ├── main.rs             # 入口分流与单实例控制
│   ├── monitor.rs          # 进程扫描、端口探测与遥测
│   └── ui.rs               # Ratatui TUI 渲染器
├── ui/                     # Slint 界面声明
│   └── cockpit.slint       # 桌面端界面定义
├── antigravity-cockpit.exe  # 发布版可执行文件
├── build.rs                # PE 资源与 Slint 构建
├── Cargo.toml              # 项目依赖配置
├── AGENTS.md               # 作业规程与工程纪律
├── tools/                  # 规程自动化检查工具
├── start.bat               # GUI 一键启动脚本
└── start_tui.bat           # TUI 一键启动脚本
```

自检（AGENTS.md 规程机器检查）：

```powershell
python tools/scan_comment.py
python tools/check_front_hygiene.py
```
