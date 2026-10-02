# Antigravity 多实例管理器作业规程

`D:\AI\multi-antigravity-rust`：Rust + Slint 1.18 桌面端 Antigravity 多实例管理器，负责把 Google Antigravity 的**第二个实例**派生到独立沙箱里跑，并监控双实例。
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

### 2. 克制白描，说人话（界面、文档与日志）

核心哲学：**客观、克制、白描**。只陈述事实与结果，不加戏、不教导、不向上汇报，坚决摒弃技术装腔。
本条适用于所有给人看的文字：UI 界面、命令行提示、Toast/弹窗、文档 README、诊断日志。

1. **拒绝概念通胀，禁止大词包装**：
   - 严禁堆砌营销级与绝对化修饰词（如“极致”、“彻底”、“神级”、“企业级”、“物理级”）。
   - 讲机制不讲噱头：写真实机制，不造玄学大词；用最平实的工程事实陈述，自信源于代码本身，不靠形容词壮胆。
2. **结果导向，禁止报备与邀功**：
   - 界面、回执与 Toast 只告诉用户「最终结果」，不向上级汇报内部执行路径（严禁写“已调用底层接口触发…”、“服务正自动拉起…”）。
   - 报错只讲具体阻断原因，严禁掺杂自我安抚与免责式废话（如“操作已中止，核心系统未受任何影响”）。
   - 动词与短语一律用白话通识（“启动/停止/重启/退出”），禁止生造冷门借代词（如把“重启”写成“回收”，把“后台独立运行”写成“脱机运行”）。
3. **日志具备诊断价值，拒绝表演式输出**：
   - 诊断日志的唯一意义是**事后排查与故障定位**，只记录确定性事实：时间、对象、PID/端口/路径、状态码、关键耗时、错误原文。
   - 严禁打印毫无信息增益的赛博表演日志（如一连串“XX 模块就绪”、“通道已启用”、“守护逻辑已载入”）。
4. **界面零说明字（Zero UI Noise）**：
   - 界面只写「它是什么（字段/状态）」和「出了什么问题（错误具体原因）」，坚决不写「怎么用（引导/解释/教导）」。
   - 全界面严格仅允许五类文案：字段名 · 必填标记 `*` · 出错的具体原因 · 禁用态的原因 · 空状态占位提示。

**机器强制**：`python tools/check_front_hygiene.py`。
注意它是**关键词黑名单**，只挡已知说法；中英文都要过一遍（英文文案扫不到），
新增文案仍要人工判断，别因为扫描绿了就放行。

### 3. 非源码一律进 `.scratch/`

报告、备份、脚本、截图、临时中间物全进 `.scratch/`（整体不进版本库）。
根目录只允许：源码（`src/`、`ui/`）、文档与设计稿（`docs/`）、资源（`assets/`）、仓库元文件
（`README.md`、`AGENTS.md`、`Cargo.toml`、`Cargo.lock`、`build.rs`、`.gitignore`、
`tools/`、`start.bat`、`start_tui.bat`）、`target/`、`data/`、`multi-antigravity.exe`
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
- **进程判定要排除自己**：发布 exe 叫 `multi-antigravity.exe`，
  任何按进程名 `contains("antigravity")` 的匹配都要排除 `multi`，避免把自己算成主机实例。
- **状态只信扫描结果**：分身的「是否运行」以是否真扫到进程为准，
  不要把「上次 spawn 记下的 PID」当成运行状态的依据 —— 那会让分身关掉后仍显示运行中。
- **不要动 `data/instance_2/home/.gemini/`**：里面有真实的 OAuth token。
  `data/` 下能安全清的只有分身关闭后的 `Cache` / `Code Cache`。
- **UI 基准是 1900×1040 客户区**（1080P 满屏），字号下限 12px，正文 13px，KPI 主数值 27px。
- **内容区铺满窗口宽度**，不设宽度上限；左右留白靠内边距给：页面 48px、header 内 32px。
  这样右上角的三个状态块（渲染状态 / 内存开销 / 时钟）在 header 内右侧，**距窗口右边恒定 80px**，
  任何窗口宽度下都不位移、不压线。
  **别再尝试"内容限宽居中 + 三块贴窗口右上角"**：内容一居中，header 卡片右边界就跟着内缩，
  三块要么被带着往里跑，要么（做成浮层后）压在卡片边框上。2026-10-02 在这里来回改了五轮。
  改字号或容器尺寸前先看 `ui/cockpit.slint` 顶部的组件定义，别单独动某一处 —— 纵向预算刚好排满，
  7 个区块的高度是算过的（标题栏 40 + 内边距 48 + header 64 + KPI 110 + 实例卡 260 + 下半区自适应
  + 工具栏 40 + 状态条 36 + 间距 80）。视觉稿在 `docs/cockpit-ui-prototype.html`。
- **不要动 `trim_working_set()` 在缩托盘时的那次调用**（用户明确要过的能力）。
  已经删掉的是定时器里每 30 秒那次 —— 实测工作集 3.4 MB / 专用内存 9.1 MB / 峰值 28.4 MB，
  裁剪只是把页面挤出工作集，真实占用一分没少，随后每秒的遥测又要硬缺页换回来。别再加回去。
- **交付真实性与热更新进程接管授权（防假交付）**：
  核心原则：**「代码写完」不等于「生效交付」**。在桌面应用、后台服务与常驻守护进程开发中，用户通常习惯将程序固定到**系统开始菜单磁贴、桌面快捷方式或常驻服务路径**，以便一键唤起即开即测。必须以**目标路径的二进制产物完成原位物理覆盖、且加载运行最新代码**为交付闭环。严禁新建别名副本（如 `_new.exe`）或仅将产物留在构建目录（如 `target/release/`），否则用户从固定入口拉起时必定仍是旧版本，造成对接信息严重错位。
  1. **严格区分「源码编辑」与「运行态生效」**：严禁将“代码已修改”等同于“已生效完成”。
  2. **热更新进程接管授权（主动解除文件锁与端口占用）**：
     - **正式接管授权**：在覆盖根目录/目标路径二进制（如 `multi-antigravity.exe`）或启动测试实例时，若遭遇旧实例正在运行引发的文件锁（Windows `WinError 32` / `Device or resource busy` / `LNK1104`）或端口占用（`EADDRINUSE`），**AI 获得正式授权主动接管并终止占用资源的旧进程**（精准定位目标 PID 执行 `taskkill /PID <pid> /F` 或相应系统命令）。
     - **原位物理覆盖**：解除占用后，必须立即完成原目标路径文件的覆盖写入，确保用户按快捷键或点击磁贴时直接调起最新构建产物。
     - **严格限定接管边界**：仅限终止由本项目自身构建运行的旧版进程，严禁扩大范围，绝不触碰用户宿主环境或其他系统关键进程。
     - **如实通报执行结果**：触发接管后，回执中必须如实陈述事实（例如 `已自动终止旧版进程（PID: xxx）并完成原位覆盖交付`）。若遭遇非本项目进程占用或系统级无法解除的锁，方触发阻断警示协议置顶通报占用者 PID。
  3. **验收凭证原则（拒绝幽灵验证）**：自测必须有新产物运行的确定性证据（如新 PID、新启动日志、新版本号），严禁在旧进程的残余输出上做假验证。

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
python tools/ui_probe.py max <pid>         # 最大化，用来验证超宽屏下的布局
python tools/ui_probe.py shot <pid>        # 截窗口内容到 .scratch/（被别的窗口挡着也能截）
                                           #   可加裁剪区：shot <pid> 2 name.png 20,270,1860,300
python tools/ui_probe.py key <pid> 52 20   # 发按键（52 = R，分身没跑时只写日志，适合刷日志量）
python tools/ui_probe.py kill <pid>
```

  `shot` 走 PrintWindow，比抓屏可靠；`key` 走 PostMessage，不会打扰前台程序。
- **发布产物原位覆盖**：发布产物为根目录 `multi-antigravity.exe`（由 `target/release/multi-antigravity-rust.exe` 复制并重命名）。用户可将该 exe 固定至 Windows 10 开始菜单磁贴进行快速测试。若旧版正在运行导致覆盖被锁，必须依「热更新进程接管授权」直接终止旧版管理器进程并完成覆盖，严禁留在 `target/release/` 或生成副本，确保用户通过磁贴调起即为最新版。

## 环境事实

- Rust 1.98.1 + MSVC（`~/.cargo/bin`）。
- **Bash 工具的 PATH 被破坏**，每条命令前要
  `export PATH="/c/Users/GVH/.workbuddy/binaries/PortableGit/versions/1.2.0/usr/bin:$PATH"`，
  cargo 要 `~/.cargo/bin/cargo`。
- **默认渲染后端是软件光栅**：`main.rs` 里没设 `SLINT_BACKEND` 时强制 `winit-software`。
  界面文案别写「硬件加速 / GPU」，与事实不符。
- **发布流程**：`cargo build --release` → 复制
  `target\release\multi-antigravity-rust.exe` 到根目录改名 `multi-antigravity.exe`
  （README 里也写了这步）。exe 不进版本库。
- **`target/` 5.2 GB 不动**：D 盘剩 450 GB，占 1.2%；清掉要换 2~5 分钟全量重编（增量只要 20 秒 / 57 秒）。
  真要省就只删 `target/debug`。
- Slint 1.18 里 `ScrollView` 的 `viewport-y` **已废弃**，用 `content-y`（`viewport-height` → `content-height`）。
- **Slint 的 `VerticalLayout` 默认把剩余高度平均分配给子元素**：容器高度写死且大于内容时，
  行距会被撑开、且按子元素类型分配得还不均匀（含进度条的行涨得少）。内容要紧凑就给它加
  `alignment: start;`，或者干脆别写死容器高度、让它按内容自适应。
  实例卡踩过：固定 260px + 四行参数 → 行距变成 80/80/21。
- **`ScrollView` 里内容不足时不会贴顶**（有固定偏移）。日志区已改成 `clip: true` 容器 +
  `y: min(0px, parent.height - self.height)`：内容少贴顶、超出自动贴底显示最新行。
- Bash 里不能出现 "PowerShell" 字样（会被安全策略拦），要跑就用 PowerShell 工具。

## 边界

只做「调度与监控」：派生、隔离、遥测、凭据清理。
**不碰主机实例的凭据**（`clear_token` 只删沙箱里那一份），不接管分身内部的账号体系，
不上传任何遥测数据。
