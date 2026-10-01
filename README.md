# ⚡ Antigravity 开发者座舱 (Antigravity Studio Cockpit)

用 Rust 原生打造的 Google Antigravity 高性能双开调度与环境隔离系统。支持现代化桌面 GUI 客户端与极客终端 TUI 双模运行。

---

## 🌟 核心特性

- **现代极客原生 GUI 客户端（默认运行）**：
  - 基于 **Slint 纯原生硬件加速引擎** 构建，**彻底摒弃 Chromium 与 WebView2**，零外部浏览器运行时依赖。
  - **极致超低内存开销**：单进程物理内存开销仅 **~12 MB**，无任何附加子进程，冷启动耗时 **< 15ms**。
  - **目录零污染**：100% 纯机器码自绘，绝不在本地目录生成任何 `.WebView2` 或浏览器缓存文件。
  - **点击关闭缩小至系统托盘**：点击窗口右上角 `[✕]` 自动最小化至 Windows 系统通知区托盘常驻，避免意外关闭中断后台工作；单击/双击托盘图标秒级唤回并置顶座舱；右键托盘支持快捷唤出/调度分身。
  - **100% 离线自包含**：零外部 CDN / 字体网络依赖，断网弱网环境下依然毫秒级渲染。

- **极客终端 TUI 模式（支持 `--tui` 参数）**：
  - 基于 Ratatui 构建的单屏紧凑仪表盘，22~24 行单屏尽收眼底，无需滚动浏览。
  - 双实例实时 CPU / 物理内存 (RSS) 采集与本地语言服务端口探测。

- **Win32 进程完全脱钩 (Detached)**：
  - 采用 `CREATE_BREAKAWAY_FROM_JOB (0x01000000)` 与 `DETACHED_PROCESS (0x00000008)` 独立派生子进程树。
  - **关闭座舱绝不影响已启动的分身运行**，随时退出座舱，随时重新打开接管。

- **物理级环境沙箱隔离**：
  - `USERPROFILE` & `HOME` 隔离到 `data/instance_2/home`，彻底杜绝 `.gemini` 配置文件碰撞。
  - `APPDATA` 隔离到 `data/instance_2/AppData/Roaming`，独立插件与窗口缓存。
  - `SSH_CONNECTION=127.0.0.1 50000 127.0.0.1 22` 旁路钩子：强制语言服务使用独立文件凭据 (`jetski-standalone-oauth-token`)，不污染主号的 Windows 凭据管理器。
  - `LOCALAPPDATA` 保留原生系统路径：分身调用系统 Chrome 浏览器授权时，完美继承已有的 Chrome 多用户 Profile，无需重新登录浏览器。

---

## 🚀 快速启动

### 1. 桌面 GUI 客户端（推荐）
直接双击运行根目录下的可执行文件或批处理脚本：
```text
antigravity-cockpit.exe
或
start.bat
```

### 2. 终端 TUI 仪表盘
如果需要在命令行或终端中以纯字符模式运行：
```powershell
.\antigravity-cockpit.exe --tui
或
双击运行 start_tui.bat
```

### 3. 从源码编译
```powershell
cargo build --release
```
编译产物位于 `target/release/multi-antigravity-rust.exe`，已通过 Windows PE 资源嵌入应用图标。

---

## ⌨️ 快捷操作矩阵

| 按键 | 功能 | 说明 |
| :---: | :--- | :--- |
| **<kbd>Space</kbd>** | **启动 / 唤醒分身** | 未运行时独立启动分身；运行中时调用 Win32 `SetForegroundWindow` 将分身窗口置顶激活 |
| **<kbd>K</kbd>** | **停止分身** | 优雅终止分身进程树，主机实例不受任何干扰 |
| **<kbd>R</kbd>** | **重启服务引擎** | 热回收并刷新语言服务本地端口 |
| **<kbd>C</kbd>** | **换号清空凭证** | 一键擦除独立 Token 文件，下次打开时 Chrome 会再次弹出 Google 账号选择页 |
| **<kbd>O</kbd>** | **打开沙箱目录** | 在 Windows 资源管理器中弹出 `data/instance_2` 数据目录 |
| **<kbd>M</kbd>** | **深浅主题切换** | 曜石暗色版与凝雪浅色版毫秒级即时热切 (GUI 模式) |
| **<kbd>T</kbd>** | **中 / EN 切换** | 纯中文界面与纯英文界面瞬时切换 |
| **<kbd>L</kbd>** | **控制台清屏** | 清空诊断日志缓冲区 |
| **<kbd>Q</kbd>** | **彻底退出座舱** | 完全释放系统资源并退出应用（点击 `[✕]` 为最小化至托盘） |

---

## 📁 目录结构

```text
multi-antigravity-rust/
├── assets/                 # 应用静态资产（图标源图、.ico、托盘裸数据）
│   ├── icon.ico            # Windows PE 资源图标
│   ├── icon.png            # 1024x1024 高清母版
│   └── icon_32.rgba        # 托盘内嵌像素矩阵
├── src/                    # Rust 核心源代码
│   ├── app.rs              # TUI 应用状态机与事件调度
│   ├── gui.rs              # Slint 原生 GUI 桌面客户端与系统托盘
│   ├── i18n.rs             # 纯中/英多语言文案字典
│   ├── launcher.rs         # Win32 进程派生、凭据擦除与沙箱隔离内核
│   ├── main.rs             # CLI / GUI 模式分流与 Windows 子系统配置
│   ├── monitor.rs          # 实例进程扫描、端口探测与内存遥测
│   └── ui.rs               # Ratatui TUI 布局渲染器
├── ui/                     # 现代声明式 UI 规范
│   └── cockpit.slint       # Slint 原生矢量界面声明
├── antigravity-cockpit.exe  # 编译发布的 Windows 原生客户端 (零 Chromium)
├── build.rs                # Windows PE 资源编译 (winres) 与 Slint 编译 (slint-build)
├── Cargo.toml              # 项目依赖配置
├── start.bat               # 原生 GUI 一键启动脚本
└── start_tui.bat           # 终端 TUI 一键启动脚本
```
