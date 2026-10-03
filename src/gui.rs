use crate::launcher::LauncherConfig;
use crate::monitor::TelemetryMonitor;
use anyhow::Result;
use slint::{CloseRequestResponse, ComponentHandle, ModelRc, Timer, TimerMode, VecModel};
use std::sync::{Arc, Mutex};

slint::include_modules!();

fn update_ui_log_lines(ui: &MainWindow, st: &crate::streamer::LogStreamer) {
    let parsed = st.get_parsed_lines();
    ui.set_log_line_count(st.buffer.len() as i32);
    let items: Vec<LogLineData> = parsed
        .into_iter()
        .map(|p| LogLineData {
            line_no: p.line_no.into(),
            time_str: p.time_str.into(),
            level: p.level.into(),
            level_color: slint::Color::from_rgb_u8(p.level_color_rgb.0, p.level_color_rgb.1, p.level_color_rgb.2),
            tag: p.tag.into(),
            tag_color: slint::Color::from_rgb_u8(p.tag_color_rgb.0, p.tag_color_rgb.1, p.tag_color_rgb.2),
            text: p.text.into(),
            text_color: slint::Color::from_rgb_u8(p.text_color_rgb.0, p.text_color_rgb.1, p.text_color_rgb.2),
        })
        .collect();
    let model: ModelRc<LogLineData> = std::rc::Rc::new(VecModel::from(items)).into();
    ui.set_log_lines(model);
}

pub fn run_gui(activation_event: crate::ActivationEvent) -> Result<()> {
    let config = Arc::new(Mutex::new(LauncherConfig::new()?));
    let sub_pid = Arc::new(Mutex::new(None::<u32>));
    let monitor = Arc::new(Mutex::new(TelemetryMonitor::new()));

    let main_window = MainWindow::new()?;
    main_window.set_app_title("Antigravity 多实例管理器".into());
    main_window.set_app_subtitle("环境隔离与进程监控".into());
    main_window.set_app_version(env!("CARGO_PKG_VERSION").into());

    const ICON_RGBA: &[u8] = include_bytes!("../assets/icon_32.rgba");
    let tray_menu = tray_icon::menu::Menu::new();
    let item_show = tray_icon::menu::MenuItem::new("显示管理器", true, None);
    let item_sub = tray_icon::menu::MenuItem::new("启动/唤醒分身", true, None);
    let sep = tray_icon::menu::PredefinedMenuItem::separator();
    let item_quit = tray_icon::menu::MenuItem::new("退出管理器", true, None);

    let show_id = item_show.id().clone();
    let sub_id = item_sub.id().clone();
    let quit_id = item_quit.id().clone();

    tray_menu.append_items(&[&item_show, &item_sub, &sep, &item_quit])?;
    let tray_icon_img = tray_icon::Icon::from_rgba(ICON_RGBA.to_vec(), 32, 32)?;
    let _tray = tray_icon::TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_menu_on_left_click(false)
        .with_tooltip(format!("Antigravity 多实例管理器 v{}", env!("CARGO_PKG_VERSION")))
        .with_icon(tray_icon_img)
        .build()?;

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
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetWindowThreadProcessId,
    };

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> i32 {
        unsafe {
            let mut proc_id = 0u32;
            GetWindowThreadProcessId(hwnd, &mut proc_id);
            if proc_id == GetCurrentProcessId() {
                let mut class_buf = [0u16; 64];
                let len = GetClassNameW(hwnd, class_buf.as_mut_ptr(), class_buf.len() as i32);
                if len > 0 {
                    let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);
                    if class_name == "Window Class" {
                        let out_ptr = lparam as *mut HWND;
                        *out_ptr = hwnd;
                        return 0;
                    }
                }
            }
            1
        }
    }

    let mut found_hwnd: HWND = std::ptr::null_mut();
    unsafe {
        EnumWindows(Some(enum_proc), &mut found_hwnd as *mut _ as LPARAM);
    }
    found_hwnd
}

#[cfg(target_os = "windows")]
fn apply_dark_window_attributes(hwnd: windows_sys::Win32::Foundation::HWND) {
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_CAPTION_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE,
    };
    use windows_sys::Win32::Graphics::Gdi::CreateSolidBrush;
    use windows_sys::Win32::UI::WindowsAndMessaging::{SetClassLongPtrW, GCLP_HBRBACKGROUND};

    if hwnd.is_null() {
        return;
    }
    unsafe {
        let dark = 1i32;
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            &dark as *const _ as *const _,
            std::mem::size_of::<i32>() as u32,
        );
        let caption_color = 0x000E0907u32;
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_COLOR as u32,
            &caption_color as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        );
        let brush = CreateSolidBrush(0x000E0907);
        if !brush.is_null() {
            SetClassLongPtrW(hwnd, GCLP_HBRBACKGROUND, brush as _);
        }
    }
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
        let cur = ui.get_redraw_tick();
        ui.set_redraw_tick(if cur == 0 { 1 } else { 0 });
        let window = ui.window();
        let _ = window.show();

        let hwnd = get_cockpit_hwnd();
        if !hwnd.is_null() {
            apply_dark_window_attributes(hwnd);
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

    main_window.window().on_close_requested({
        let ui_weak = main_window.as_weak();
        move || {
            #[cfg(target_os = "windows")]
            hide_cockpit_to_tray(&ui_weak);
            CloseRequestResponse::KeepWindowShown
        }
    });

    let streamer = Arc::new(Mutex::new(crate::streamer::LogStreamer::new()));
    {
        let st = streamer.lock().unwrap();
        main_window.set_current_stream_idx(st.get_source_index());
        main_window.set_stream_source_name(st.current_source.display_name().into());
        update_ui_log_lines(&main_window, &st);
    }
    main_window.set_token_present(config.lock().unwrap().is_token_present());


    let append_log = {
        let streamer = Arc::clone(&streamer);
        move |ui: &MainWindow, tag: &str, msg: &str| {
            let mut st = streamer.lock().unwrap();
            st.add_system_log(tag, msg);
            if st.current_source == crate::streamer::StreamSource::CockpitSystem {
                update_ui_log_lines(ui, &st);
            }
        }
    };

    main_window.on_cycle_stream_source({
        let ui_weak = main_window.as_weak();
        let streamer = Arc::clone(&streamer);
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut st = streamer.lock().unwrap();
                let next = st.cycle_source();
                let idx = st.get_source_index();
                ui.set_current_stream_idx(idx);
                ui.set_stream_source_name(next.display_name().into());
                update_ui_log_lines(&ui, &st);
                ui.set_toast_message(format!("日志管道已切换为: {}", next.display_name()).into());
            }
        }
    });

    main_window.on_select_stream_source({
        let ui_weak = main_window.as_weak();
        let streamer = Arc::clone(&streamer);
        move |idx| {
            if let Some(ui) = ui_weak.upgrade() {
                let mut st = streamer.lock().unwrap();
                let source = crate::streamer::LogStreamer::from_source_index(idx);
                st.set_source(source);
                ui.set_current_stream_idx(idx);
                ui.set_stream_source_name(source.display_name().into());
                update_ui_log_lines(&ui, &st);
                ui.set_toast_message(format!("已切换至: {}", source.display_name()).into());
            }
        }
    });

    main_window.on_action_clear_log({
        let ui_weak = main_window.as_weak();
        let streamer = Arc::clone(&streamer);
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut st = streamer.lock().unwrap();
                st.clear_current();
                update_ui_log_lines(&ui, &st);
                ui.set_toast_message("当前日志管道已清空".into());
            }
        }
    });

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
                    ui.set_toast_message(format!("已唤醒分身窗口 (PID: {})。", pid).into());
                    append_log(&ui, "WIN32", &format!("唤醒分身窗口 (PID: {})", pid));
                } else {
                    match c.spawn_detached() {
                        Ok(new_pid) => {
                            *p = Some(new_pid);
                            ui.set_sub_running(true);
                            ui.set_sub_pid(new_pid.to_string().into());
                            ui.set_toast_message(format!("分身已在后台启动 (PID: {})。", new_pid).into());
                            append_log(&ui, "SPAWN", &format!("分身已在后台启动 (PID: {})", new_pid));
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
                    format!("语言服务已重启 (PID: {})。", pid).into(),
                );
                append_log(&ui, "LANG_SVR", &format!("重启语言服务 (PID: {})", pid));
            } else {
                ui.set_toast_message("语言服务未在运行。".into());
                append_log(&ui, "LANG_SVR", "语言服务未在运行");
            }
        }
    });

    main_window.on_action_clear({
        let ui_weak = main_window.as_weak();
        let conf = Arc::clone(&config);
        let append_log = append_log.clone();
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                let c = conf.lock().unwrap();
                match c.clear_token() {
                    Ok(true) => {
                        ui.set_toast_message("独立凭据已清除。".into());
                        append_log(&ui, "AUTH", "独立凭据已清除");
                    }
                    Ok(false) => {
                        ui.set_toast_message("未发现独立凭据。".into());
                    }
                    Err(e) => {
                        ui.set_toast_message(format!("清空凭据失败: {}", e).into());
                    }
                }
            }
        }
    });

    main_window.on_action_open({
        let ui_weak = main_window.as_weak();
        let conf = Arc::clone(&config);
        let append_log = append_log.clone();
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                let c = conf.lock().unwrap();
                let _ = c.open_sandbox_in_explorer();
                ui.set_toast_message("已打开沙箱目录。".into());
                append_log(&ui, "EXPLORER", "打开沙箱目录 data/instance_2");
            }
        }
    });

    main_window.on_action_proxy({
        let ui_weak = main_window.as_weak();
        let append_log = append_log.clone();
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                let status = crate::proxy::ProxyManager::check_status();
                if status.is_any_deployed() {
                    match crate::proxy::ProxyManager::remove_all() {
                        Ok(msg) => {
                            ui.set_toast_message("已移除免 TUN 代理".into());
                            append_log(&ui, "PROXY", &msg);
                        }
                        Err(e) => {
                            let err = format!("移除失败: {}", e);
                            ui.set_toast_message(err.clone().into());
                            append_log(&ui, "ERROR", &err);
                        }
                    }
                } else {
                    match crate::proxy::ProxyManager::deploy_all() {
                        Ok(msg) => {
                            ui.set_toast_message("免 TUN 代理已部署就绪".into());
                            append_log(&ui, "PROXY", &msg);
                            if !status.port_online {
                                append_log(&ui, "WARN", "本地 7890 端口未监听。");
                            }
                        }
                        Err(e) => {
                            let err = format!("部署失败: {}", e);
                            ui.set_toast_message(err.clone().into());
                            append_log(&ui, "ERROR", &err);
                        }
                    }
                }
            }
        }
    });

    main_window.on_action_quit({
        move || {
            std::process::exit(0);
        }
    });

    let log_timer = Timer::default();
    let ui_weak_log = main_window.as_weak();
    let streamer_poll = Arc::clone(&streamer);
    log_timer.start(TimerMode::Repeated, std::time::Duration::from_millis(500), move || {
        if let Some(ui) = ui_weak_log.upgrade() {
            let mut st = streamer_poll.lock().unwrap();
            if st.poll_updates() {
                update_ui_log_lines(&ui, &st);
            }
        }
    });

    let timer = Timer::default();
    let ui_weak_mon = main_window.as_weak();
    let ui_weak_activate = main_window.as_weak();
    let monitor_timer = Arc::clone(&monitor);
    let sub_pid_mon = Arc::clone(&sub_pid);
    let config_timer = Arc::clone(&config);

    let mut self_sys = sysinfo::System::new();
    #[cfg(windows)]
    let restore_timer = Timer::default();
    #[cfg(windows)]
    let ui_weak_restore = main_window.as_weak();
    #[cfg(windows)]
    let mut was_iconic = false;
    #[cfg(windows)]
    let mut configured_dark = false;

    #[cfg(windows)]
    restore_timer.start(TimerMode::Repeated, std::time::Duration::from_millis(50), move || {
        let hwnd = get_cockpit_hwnd();
        if hwnd.is_null() {
            return;
        }
        if !configured_dark {
            apply_dark_window_attributes(hwnd);
            configured_dark = true;
        }
        let is_iconic = unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::IsIconic(hwnd) != 0
        };
        if was_iconic && !is_iconic {
            let Some(ui) = ui_weak_restore.upgrade() else {
                was_iconic = is_iconic;
                return;
            };
            let cur = ui.get_redraw_tick();
            ui.set_redraw_tick(if cur == 0 { 1 } else { 0 });
            ui.window().request_redraw();
        }
        was_iconic = is_iconic;
    });

    timer.start(TimerMode::Repeated, std::time::Duration::from_millis(1000), move || {
        if let Some(ui) = ui_weak_mon.upgrade() {
            #[cfg(windows)]
            {
                if unsafe {
                    windows_sys::Win32::System::Threading::WaitForSingleObject(activation_event, 0)
                } == 0
                {
                    show_cockpit_from_tray(&ui_weak_activate);
                }
            }

            let mut mon = monitor_timer.lock().unwrap();
            let known_sub = *sub_pid_mon.lock().unwrap();
            mon.refresh(known_sub);
            if let Some(pid) = mon.sub.electron_pid {
                *sub_pid_mon.lock().unwrap() = Some(pid);
            } else if !mon.sub.is_running {
                *sub_pid_mon.lock().unwrap() = None;
            }


            ui.set_host_running(mon.host.is_running);
            ui.set_host_cpu(mon.host.cpu_usage);
            ui.set_host_ram(mon.host.memory_rss_mb as f32);
            if let Some(port) = mon.host.ls_port {
                ui.set_host_port(port.to_string().into());
            } else {
                ui.set_host_port("--".into());
            }

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

            ui.set_total_ram(format!("{:.1} MB", mon.total_memory_mb).into());
            ui.set_current_time(chrono::Local::now().format("%H:%M:%S").to_string().into());

            let self_pid = sysinfo::Pid::from_u32(std::process::id());
            self_sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[self_pid]), true);
            if let Some(proc) = self_sys.process(self_pid) {
                ui.set_ram_overhead(
                    format!("开销: {:.1} MB", proc.memory() as f64 / (1024.0 * 1024.0)).into(),
                );
            }
            ui.set_token_present(config_timer.lock().unwrap().is_token_present());


            let p_status = crate::proxy::ProxyManager::check_status();
            if p_status.is_fully_deployed() {
                if p_status.port_online {
                    ui.set_proxy_status("免TUN代理就绪 (7890)".into());
                    ui.set_proxy_color(slint::Color::from_rgb_u8(16, 185, 129));
                } else {
                    ui.set_proxy_status("代理已就绪(端口未开)".into());
                    ui.set_proxy_color(slint::Color::from_rgb_u8(245, 158, 11));
                }
                ui.set_proxy_btn_text("移除免TUN代理".into());
                ui.set_proxy_btn_color(slint::Color::from_rgb_u8(244, 63, 94));
            } else if p_status.is_any_deployed() {
                ui.set_proxy_status("代理部分就绪".into());
                ui.set_proxy_color(slint::Color::from_rgb_u8(245, 158, 11));
                ui.set_proxy_btn_text("重新部署代理".into());
                ui.set_proxy_btn_color(slint::Color::from_rgb_u8(251, 191, 36));
            } else {
                ui.set_proxy_status("未部署免TUN代理".into());
                ui.set_proxy_color(slint::Color::from_rgb_u8(100, 116, 139));
                ui.set_proxy_btn_text("部署免TUN代理".into());
                ui.set_proxy_btn_color(slint::Color::from_rgb_u8(16, 185, 129));
            }
        }
    });

    let sandbox_path = match config.lock() {
        Ok(c) => {
            let path = c.sandbox_root.display().to_string();
            main_window.set_sandbox_root(path.clone().into());
            path
        }
        Err(_) => "--".to_string(),
    };

    {
        let host_line = {
            let mut mon = monitor.lock().unwrap();
            mon.refresh(None);
            if mon.host.is_running {
                match mon.host.electron_pid {
                    Some(pid) => format!("主机实例已识别 (PID: {pid})"),
                    None => "主机实例已识别".to_string(),
                }
            } else {
                "未检测到主机实例进程".to_string()
            }
        };
        append_log(&main_window, "SYSTEM", "座舱就绪");
        append_log(&main_window, "SANDBOX", &format!("沙箱根目录: {sandbox_path}"));
        append_log(&main_window, "MONITOR", &host_line);

        let initial_proxy = crate::proxy::ProxyManager::check_status();
        if initial_proxy.is_fully_deployed() {
            main_window.set_proxy_status(if initial_proxy.port_online { "免TUN代理就绪 (7890)".into() } else { "代理已就绪(端口未开)".into() });
            main_window.set_proxy_color(if initial_proxy.port_online { slint::Color::from_rgb_u8(16, 185, 129) } else { slint::Color::from_rgb_u8(245, 158, 11) });
            main_window.set_proxy_btn_text("移除免TUN代理".into());
            main_window.set_proxy_btn_color(slint::Color::from_rgb_u8(244, 63, 94));
            append_log(&main_window, "PROXY", &format!("免 TUN 代理已就绪 (本地 7890 端口: {})", if initial_proxy.port_online { "在线" } else { "未检测到监听" }));
        } else {
            main_window.set_proxy_status("未部署免TUN代理".into());
            main_window.set_proxy_color(slint::Color::from_rgb_u8(100, 116, 139));
            main_window.set_proxy_btn_text("部署免TUN代理".into());
            main_window.set_proxy_btn_color(slint::Color::from_rgb_u8(16, 185, 129));
            append_log(&main_window, "PROXY", "免 TUN 代理: 未部署");
        }
    }

    main_window.show()?;
    #[cfg(target_os = "windows")]
    {
        let hwnd = get_cockpit_hwnd();
        apply_dark_window_attributes(hwnd);
    }
    let _ = slint::run_event_loop_until_quit();
    Ok(())
}
