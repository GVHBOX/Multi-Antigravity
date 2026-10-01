use crate::launcher::LauncherConfig;
use crate::monitor::TelemetryMonitor;
use anyhow::Result;
use slint::{CloseRequestResponse, ComponentHandle, Timer, TimerMode};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

slint::include_modules!();

pub fn run_gui() -> Result<()> {
    let config = Arc::new(Mutex::new(LauncherConfig::new()?));
    let sub_pid = Arc::new(Mutex::new(None::<u32>));
    let monitor = Arc::new(Mutex::new(TelemetryMonitor::new()));

    let main_window = MainWindow::new()?;

    // 1. 系统托盘与右键菜单构建
    const ICON_RGBA: &[u8] = include_bytes!("../assets/icon_32.rgba");
    let tray_menu = tray_icon::menu::Menu::new();
    let item_show = tray_icon::menu::MenuItem::new("显示座舱 (Show Cockpit)", true, None);
    let item_sub = tray_icon::menu::MenuItem::new("启动/调出分身 (Launch Sub-instance)", true, None);
    let sep = tray_icon::menu::PredefinedMenuItem::separator();
    let item_quit = tray_icon::menu::MenuItem::new("彻底退出 (Exit)", true, None);

    let show_id = item_show.id().clone();
    let sub_id = item_sub.id().clone();
    let quit_id = item_quit.id().clone();

    tray_menu.append_items(&[&item_show, &item_sub, &sep, &item_quit])?;
    let tray_icon_img = tray_icon::Icon::from_rgba(ICON_RGBA.to_vec(), 32, 32)?;
    let _tray = tray_icon::TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_menu_on_left_click(false)
        .with_tooltip("Antigravity 开发者座舱 · Studio Cockpit (Slint Native)")
        .with_icon(tray_icon_img)
        .build()?;

    // 2. 托盘事件监听与跨线程派发
    let config_tray = Arc::clone(&config);
    let sub_pid_tray = Arc::clone(&sub_pid);
    let ui_weak_tray = main_window.as_weak();

    tray_icon::TrayIconEvent::set_event_handler(Some({
        let ui_weak = ui_weak_tray.clone();
        move |event| {
            if let tray_icon::TrayIconEvent::Click {
                button: tray_icon::MouseButton::Left,
                button_state: tray_icon::MouseButtonState::Up,
                ..
            }
            | tray_icon::TrayIconEvent::DoubleClick {
                button: tray_icon::MouseButton::Left,
                ..
            } = event
            {
                #[cfg(target_os = "windows")]
                show_cockpit_from_tray(&ui_weak);
            }
        }
    }));

    tray_icon::menu::MenuEvent::set_event_handler(Some({
        let conf = config_tray;
        let pid_lock = sub_pid_tray;
        let ui_weak = ui_weak_tray.clone();
        move |event: tray_icon::menu::MenuEvent| {
            if event.id == show_id {
                #[cfg(target_os = "windows")]
                show_cockpit_from_tray(&ui_weak);
            } else if event.id == sub_id {
                let conf = conf.clone();
                let pid_lock = pid_lock.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    let c = conf.lock().unwrap();
                    let mut p = pid_lock.lock().unwrap();
                    if let Some(pid) = *p {
                        c.bring_to_front(pid);
                    } else if let Ok(new_pid) = c.spawn_detached() {
                        *p = Some(new_pid);
                    }
                });
            } else if event.id == quit_id {
                std::process::exit(0);
            }
        }
    }));

#[cfg(target_os = "windows")]
fn trim_working_set() {
    unsafe {
        windows_sys::Win32::System::Threading::SetProcessWorkingSetSize(
            windows_sys::Win32::System::Threading::GetCurrentProcess(),
            usize::MAX,
            usize::MAX,
        );
    }
}

#[cfg(target_os = "windows")]
fn get_cockpit_hwnd() -> windows_sys::Win32::Foundation::HWND {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId};

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> i32 {
        unsafe {
            let mut proc_id = 0u32;
            GetWindowThreadProcessId(hwnd, &mut proc_id);
            if proc_id == GetCurrentProcessId() {
                let out_ptr = lparam as *mut HWND;
                *out_ptr = hwnd;
                0 // 停止枚举，找到了
            } else {
                1 // 继续枚举
            }
        }
    }

    let mut found_hwnd: HWND = std::ptr::null_mut();
    unsafe {
        EnumWindows(Some(enum_proc), &mut found_hwnd as *mut _ as LPARAM);
    }
    found_hwnd
}

#[cfg(target_os = "windows")]
fn hide_cockpit_to_tray(ui_weak: &slint::Weak<MainWindow>) {
    if let Some(ui) = ui_weak.upgrade() {
        let _ = ui.window().hide();
    }
    trim_working_set();
}

#[cfg(target_os = "windows")]
fn show_cockpit_from_tray(ui_weak: &slint::Weak<MainWindow>) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };

    let ui_weak = ui_weak.clone();
    let _ = slint::invoke_from_event_loop(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };
        let window = ui.window();
        let _ = window.show();

        let hwnd = get_cockpit_hwnd();
        if !hwnd.is_null() {
            unsafe {
                if IsIconic(hwnd) != 0 {
                    ShowWindow(hwnd, SW_RESTORE);
                }
                SetForegroundWindow(hwnd);
            }
        }
        window.request_redraw();
    });
}

    // 3. 点击右上角 [X] 缩小至系统托盘，不退出应用并修剪工作集
    main_window.window().on_close_requested({
        let ui_weak = main_window.as_weak();
        move || {
            #[cfg(target_os = "windows")]
            hide_cockpit_to_tray(&ui_weak);
            CloseRequestResponse::KeepWindowShown
        }
    });

    // 4. 绑定 UI 回调与按键动作
    let log_buffer: Rc<RefCell<VecDeque<String>>> =
        Rc::new(RefCell::new(VecDeque::with_capacity(250)));
    let append_log = {
        let log_buffer = Rc::clone(&log_buffer);
        move |ui: &MainWindow, tag: &str, msg: &str| {
            let time = chrono::Local::now().format("%H:%M:%S").to_string();
            let mut buffer = log_buffer.borrow_mut();
            if buffer.len() >= 250 {
                buffer.pop_front();
            }
            buffer.push_back(format!("[{}] [{}] {}\n", time, tag, msg));
            ui.set_log_content(buffer.iter().cloned().collect::<String>().into());
        }
    };

    // Space
    main_window.on_action_space({
        let ui_weak = main_window.as_weak();
        let conf = Arc::clone(&config);
        let pid_lock = Arc::clone(&sub_pid);
        let append_log = append_log.clone();
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                let c = conf.lock().unwrap();
                let mut p = pid_lock.lock().unwrap();
                if let Some(pid) = *p {
                    c.bring_to_front(pid);
                    ui.set_toast_message(format!("分身正在运行 (PID: {})，已前置置顶窗口。", pid).into());
                    append_log(&ui, "WIN32", &format!("前置分身窗口 (PID: {})", pid));
                } else {
                    match c.spawn_detached() {
                        Ok(new_pid) => {
                            *p = Some(new_pid);
                            ui.set_sub_running(true);
                            ui.set_sub_pid(new_pid.to_string().into());
                            ui.set_toast_message(format!("分身已脱机启动 (PID: {})。", new_pid).into());
                            append_log(&ui, "SPAWN", &format!("分身已独立派生 (PID: {})", new_pid));
                        }
                        Err(e) => {
                            ui.set_toast_message(format!("分身启动失败: {}", e).into());
                            append_log(&ui, "ERROR", &format!("分身启动失败: {}", e));
                        }
                    }
                }
            }
        }
    });

    // K
    main_window.on_action_kill({
        let ui_weak = main_window.as_weak();
        let conf = Arc::clone(&config);
        let pid_lock = Arc::clone(&sub_pid);
        let append_log = append_log.clone();
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                let c = conf.lock().unwrap();
                let mut p = pid_lock.lock().unwrap();
                if let Some(pid) = *p {
                    let _ = c.kill_sub_instance_by_pid(pid);
                    *p = None;
                    ui.set_sub_running(false);
                    ui.set_sub_pid("--".into());
                    ui.set_sub_port("--".into());
                    ui.set_toast_message(format!("分身进程 (PID: {}) 已终止。", pid).into());
                    append_log(&ui, "KILL", &format!("分身进程 (PID: {}) 已终止", pid));
                } else {
                    ui.set_toast_message("分身未在运行。".into());
                }
            }
        }
    });

    // R
    main_window.on_action_restart({
        let ui_weak = main_window.as_weak();
        let conf = Arc::clone(&config);
        let monitor_restart = Arc::clone(&monitor);
        let append_log = append_log.clone();
        move || {
            let Some(ui) = ui_weak.upgrade() else {
                return;
            };
            let ls_pid = monitor_restart.lock().unwrap().sub.ls_pid;
            let c = conf.lock().unwrap();
            if let Some(pid) = ls_pid {
                let _ = c.kill_sub_instance_by_pid(pid);
                ui.set_toast_message(
                    format!("已回收分身语言服务 (PID: {})，Electron 会自动重新拉起。", pid).into(),
                );
                append_log(&ui, "LANG_SVR", &format!("回收分身语言服务 (PID: {})", pid));
            } else {
                ui.set_toast_message("分身语言服务未在运行，无需回收。".into());
                append_log(&ui, "LANG_SVR", "分身语言服务未在运行，无需回收");
            }
        }
    });

    // C
    main_window.on_action_clear({
        let ui_weak = main_window.as_weak();
        let conf = Arc::clone(&config);
        let append_log = append_log.clone();
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                let c = conf.lock().unwrap();
                match c.clear_token() {
                    Ok(true) => {
                        ui.set_toast_message("独立凭据文件已擦除，下次打开将重新弹出 Google 登录授权。".into());
                        append_log(&ui, "AUTH", "独立 Token 凭据已清空 (待重新授权)");
                    }
                    Ok(false) => {
                        ui.set_toast_message("未发现独立凭据文件 (已处于未授权状态)。".into());
                    }
                    Err(e) => {
                        ui.set_toast_message(format!("清空凭据失败: {}", e).into());
                    }
                }
            }
        }
    });

    // O
    main_window.on_action_open({
        let ui_weak = main_window.as_weak();
        let conf = Arc::clone(&config);
        let append_log = append_log.clone();
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                let c = conf.lock().unwrap();
                let _ = c.open_sandbox_in_explorer();
                ui.set_toast_message("已在 Windows 资源管理器中打开沙箱目录。".into());
                append_log(&ui, "EXPLORER", "弹出沙箱目录 data/instance_2");
            }
        }
    });

    // Q
    main_window.on_action_quit({
        move || {
            std::process::exit(0);
        }
    });

    // 5. 1秒高精度实时指标刷新定时器
    let timer = Timer::default();
    let ui_weak_mon = main_window.as_weak();
    let monitor_timer = Arc::clone(&monitor);
    let sub_pid_mon = Arc::clone(&sub_pid);

    let mut self_sys = sysinfo::System::new();

    timer.start(TimerMode::Repeated, std::time::Duration::from_millis(1000), move || {
        if let Some(ui) = ui_weak_mon.upgrade() {
            let mut mon = monitor_timer.lock().unwrap();
            let known_sub = *sub_pid_mon.lock().unwrap();
            mon.refresh(known_sub);
            if !mon.sub.is_running {
                *sub_pid_mon.lock().unwrap() = None;
            }

            // 更新 Host 指标
            ui.set_host_running(mon.host.is_running);
            ui.set_host_cpu(mon.host.cpu_usage);
            ui.set_host_ram(mon.host.memory_rss_mb as f32);
            if let Some(port) = mon.host.ls_port {
                ui.set_host_port(port.to_string().into());
            } else {
                ui.set_host_port("--".into());
            }

            // 更新 Sub 指标
            ui.set_sub_running(mon.sub.is_running);
            if let Some(pid) = mon.sub.electron_pid {
                ui.set_sub_pid(pid.to_string().into());
            } else {
                ui.set_sub_pid("--".into());
            }
            if let Some(port) = mon.sub.ls_port {
                ui.set_sub_port(port.to_string().into());
            } else {
                ui.set_sub_port("--".into());
            }
            ui.set_sub_cpu(mon.sub.cpu_usage);
            ui.set_sub_ram(mon.sub.memory_rss_mb as f32);

            // 更新聚合总内存与时钟
            ui.set_total_ram(format!("{:.1} MB", mon.total_memory_mb).into());
            ui.set_current_time(chrono::Local::now().format("%H:%M:%S").to_string().into());

            let self_pid = sysinfo::Pid::from_u32(std::process::id());
            self_sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[self_pid]), true);
            if let Some(proc) = self_sys.process(self_pid) {
                ui.set_ram_overhead(
                    format!("开销: {:.1} MB", proc.memory() as f64 / (1024.0 * 1024.0)).into(),
                );
            }
        }
    });

    main_window.show()?;
    let _ = slint::run_event_loop_until_quit();
    Ok(())
}
