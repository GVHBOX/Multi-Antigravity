# Antigravity 座舱作业规程

`D:\AI\multi-antigravity-rust`：Rust + Slint 1.18 桌面座舱，负责把 Google Antigravity 的**第二个实例**派生到独立沙箱里跑，并监控双实例。
两种运行形态：默认 Slint GUI（带系统托盘），`--tui` / `-t` 走 Ratatui 终端仪表盘。
本文件只讲规矩，功能介绍在 `README.md`。

## 4 条铁律

### 1. 代码零注释

核心源码 = `src/`（Rust）与 `ui/`（Slint）：不写注释，名字起清楚就行。
Rust 的 `///` / `//!` 也算注释，一样禁止；Slint 的 `//` 与 `/* */` 同样禁止。

`tools/` 不算核心源码，那里的 docstring / 注释照常写，别去清理。
例外只有：被运行时真正读取的字符串（URL、命令行参数、环境变量值）。
`#[derive]` / `#[allow(...)]` 之类的属性不算注释。
整理存量代码时，已有注释是清除对象（只清 `src/` 与 `ui/` 里的）。

**机器强制**：`tools/scan_comment.py`（`--selfcheck` 走校准件反向注入）。
校准件在 `tools/fixture-comments/`：`clean.rs` 里的 URL、raw string、字符字面量、生命周期都不得误报，
`dirty.rs` 必须命中。

```bash
python tools/scan_comment.py
python tools/scan_comment.py --selfcheck
```

### 2. 界面不加说明字

只写「它是什么」和「出了什么问题」，不写「怎么用」。禁止解释性与引导性文字。
允许五类：字段名 · 必填标记 `*` · 出错的具体原因 · 禁用态的原因 · 空状态文案。
示范格式用 placeholder，不加说明行。

这条管所有「给人看」的字符串：Slint 的 `text` / `value` / `sub_right` / `toast_message`、
托盘菜单项、TUI 的标签与回执。**诊断终端的日志流不算界面**（`append_log` / `add_log` 的内容）。

**机器强制**：`python tools/check_front_hygiene.py`。
注意它是**关键词黑名单**，只挡已知说法；中英文都要过一遍（英文文案扫不到），
新增文案仍要人工判断，别因为扫描绿了就放行。

### 3. 非源码一律进 `.scratch/`

报告、备份、脚本、截图、临时中间物全进 `.scratch/`（整体不进版本库）。
根目录只允许：源码（`src/`、`ui/`）、资源（`assets/`）、仓库元文件
（`README.md`、`AGENTS.md`、`Cargo.toml`、`Cargo.lock`、`build.rs`、`.gitignore`、
`tools/`、`start.bat`、`start_tui.bat`）、`target/`、`data/`、`antigravity-cockpit.exe`
（发布产物，`*.exe` 已被 `.gitignore` 排除，是手工复制出来的，不是构建输出）。

`assets/icon.png` 是 Slint 唯一引用的图片，**不能让 `.gitignore` 的 `*.png` 把它排除掉**
（`!assets/icon.png` 这条例外就是为此存在的，别删）。
`assets/icon.ico` 是用户自己设计的图标，只装容器，不重画不缩放。

- 交付路径用原生 Windows 路径 `D:\...`，不用 `/d/...`。
- **不往 `.workbuddy/` 写东西，也不重建它。**
- **不写跨会话的记忆 / 日志文件。** 权威在 `AGENTS.md` 与 `README.md`。

### 4. 网络与代理

本机是**系统代理**（`http://127.0.0.1:7890`），不是 TUN。
`cargo` 拉包需要时走 `HTTPS_PROXY` / `HTTP_PROXY` 环境变量。

**排查网络问题前先问用户，不要自己反复试。** 原因跨好几层（目标站点 / 代理工具 /
TUN 还是系统代理 / PAC / 安全软件 / 地区封锁），只有用户知道。
顺序：一次实测拿错误原文 → 连「程序实际用的什么出口 + 浏览器能不能开」一起告诉用户 → 等确认。
禁止：反复试探、反复重装 Runtime、把环境问题当成代码问题改代码。

## 项目纪律

- **窗口可见性必须走 Slint**：隐藏用 `window.hide()`、恢复用 `window.show()`，
  **不许直接用 Win32 的 `ShowWindow(SW_HIDE/SW_SHOW)`**。
  窗口是 `with_transparent(true)` 建的，软件渲染器按 dirty region 增量呈现；
  绕过 Slint 就跳过了它 `set_visibility` 里的整窗 `mark_dirty_region`，恢复后大面积不绘制 = 透明窗口。
  这个坑已经踩过一次（2026-10-02）。
- **主循环必须是 `slint::run_event_loop_until_quit()`**：`hide()` 会 release 一个 keepalive，
  用 `run_event_loop()` 的话最小化到托盘会直接把进程结束掉。
- **托盘回调在 tray-icon 自己的线程**：动 Slint 或窗口前必须 `slint::invoke_from_event_loop`。
- **别再用「SetWindowPos 改 1px 触发重绘」的老 hack**：会把最大化窗口的布局搞坏。
- **进程判定要排除自己**：座舱 exe 叫 `antigravity-cockpit.exe`，
  任何按进程名 `contains("antigravity")` 的匹配都会把自己算成主机实例。
- **状态只信扫描结果**：分身的「是否运行」以是否真扫到进程为准，
  不要把「上次 spawn 记下的 PID」当成运行状态的依据 —— 那会让分身关掉后仍显示运行中。
- **不要动 `data/instance_2/home/.gemini/`**：里面有真实的 OAuth token。
  `data/` 下能安全清的只有分身关闭后的 `Cache` / `Code Cache`。
- **UI 基准是 1900×1040 客户区**（1080P 满屏），字号下限 12px，正文 13px，KPI 主数值 27px。
  改字号或容器尺寸前先看 `ui/cockpit.slint` 顶部的组件定义，别单独动某一处 —— 纵向预算刚好排满，
  7 个区块的高度是算过的（标题栏 40 + 内边距 48 + header 64 + KPI 110 + 实例卡 260 + 下半区自适应
  + 工具栏 40 + 状态条 36 + 间距 80）。视觉稿在 `.scratch/cockpit-ui-prototype.html`。
- **不要动 `trim_working_set()` 在缩托盘时的那次调用**（用户明确要过的能力）。
  已经删掉的是定时器里每 30 秒那次 —— 实测工作集 3.4 MB / 专用内存 9.1 MB / 峰值 28.4 MB，
  裁剪只是把页面挤出工作集，真实占用一分没少，随后每秒的遥测又要硬缺页换回来。别再加回去。

## 测试与检查

```bash
cargo check --offline                       # 零警告才算过
cargo build --release --offline             # 产物 target/release/multi-antigravity-rust.exe
python tools/scan_comment.py                # 铁律 1
python tools/scan_comment.py --selfcheck
python tools/check_front_hygiene.py         # 铁律 2
```

- 项目目前**没有单元测试**。验收靠两步：`cargo check` 零警告 + **干净克隆能编**
  （在空目录 `git clone` 本仓后 `cargo build --release`，能过才算没把构建必需的文件漏在仓库外 ——
  `assets/icon.png` 就曾经漏过一次）。
- 改动涉及窗口 / 托盘 / 渲染时，**必须起真 exe 验一遍**，别只靠编译。用 `tools/ui_probe.py`：

```bash
python tools/ui_probe.py launch            # 起临时实例，输出 PID
python tools/ui_probe.py rect <pid>        # 读真实像素尺寸 + DPI + 逻辑尺寸
python tools/ui_probe.py shot <pid>        # 截窗口内容到 .scratch/（被别的窗口挡着也能截）
python tools/ui_probe.py key <pid> 52 20   # 发按键（52 = R，分身没跑时只写日志，适合刷日志量）
python tools/ui_probe.py kill <pid>
```

  `shot` 走 PrintWindow，比抓屏可靠；`key` 走 PostMessage，不会打扰前台程序。
- 座舱常被用户开着，根目录 `antigravity-cockpit.exe` 会占用导致覆盖失败
  （`Device or resource busy`），此时新产物先留在 `target/release/`，等用户关掉再覆盖。

## 环境事实

- Rust 1.98.1 + MSVC（`~/.cargo/bin`）。
- **Bash 工具的 PATH 被破坏**，每条命令前要
  `export PATH="/c/Users/GVH/.workbuddy/binaries/PortableGit/versions/1.2.0/usr/bin:$PATH"`，
  cargo 要 `~/.cargo/bin/cargo`。
- **默认渲染后端是软件光栅**：`main.rs` 里没设 `SLINT_BACKEND` 时强制 `winit-software`。
  界面文案别写「硬件加速 / GPU」，与事实不符。
- **发布流程**：`cargo build --release` → 复制
  `target\release\multi-antigravity-rust.exe` 到根目录改名 `antigravity-cockpit.exe`
  （README 里也写了这步）。exe 不进版本库。
- **`target/` 5.2 GB 不动**：D 盘剩 450 GB，占 1.2%；清掉要换 2~5 分钟全量重编（增量只要 20 秒 / 57 秒）。
  真要省就只删 `target/debug`。
- Slint 1.18 里 `ScrollView` 的 `viewport-y` **已废弃**，用 `content-y`。
- Bash 里不能出现 "PowerShell" 字样（会被安全策略拦），要跑就用 PowerShell 工具。

## 边界

只做「调度与监控」：派生、隔离、遥测、凭据清理。
**不碰主机实例的凭据**（`clear_token` 只删沙箱里那一份），不接管分身内部的账号体系，
不上传任何遥测数据。
