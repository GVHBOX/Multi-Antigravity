use crate::launcher::LauncherConfig;
use anyhow::Result;
use std::sync::{Arc, Mutex};
use tao::{
    dpi::LogicalSize,
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

const HTML_CONTENT: &str = include_str!("../tui_prototype.html");
const ICON_RGBA: &[u8] = include_bytes!("../assets/icon_32.rgba");

enum UserEvent {
    Tray(tray_icon::TrayIconEvent),
    Menu(tray_icon::menu::MenuEvent),
}

pub fn run_gui() -> Result<()> {
    let config = Arc::new(Mutex::new(LauncherConfig::new()?));
    let sub_pid = Arc::new(Mutex::new(None::<u32>));

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    // 1. 构建系统托盘图标与上下文右键菜单
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
        .with_tooltip("Antigravity 开发者座舱 · Studio Cockpit")
        .with_icon(tray_icon_img)
        .build()?;

    // 2. 挂载托盘事件监听与转发代理
    let proxy_tray = proxy.clone();
    tray_icon::TrayIconEvent::set_event_handler(Some(move |event| {
        let _ = proxy_tray.send_event(UserEvent::Tray(event));
    }));

    let proxy_menu = proxy.clone();
    tray_icon::menu::MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy_menu.send_event(UserEvent::Menu(event));
    }));

    // 3. 构建原生窗口（设置自定义图标）
    let win_icon = tao::window::Icon::from_rgba(ICON_RGBA.to_vec(), 32, 32).ok();
    let mut win_builder = WindowBuilder::new()
        .with_title("Antigravity 开发者座舱 · Studio Cockpit")
        .with_inner_size(LogicalSize::new(1220.0, 820.0))
        .with_min_inner_size(LogicalSize::new(960.0, 680.0));

    if let Some(icon) = win_icon {
        win_builder = win_builder.with_window_icon(Some(icon));
    }

    let window = win_builder.build(&event_loop)?;

    // 4. 构建 WebView2 视图与双向 IPC
    let config_clone = Arc::clone(&config);
    let sub_pid_clone = Arc::clone(&sub_pid);

    let _webview = WebViewBuilder::new()
        .with_html(HTML_CONTENT)
        .with_ipc_handler(move |req| {
            let cmd = req.body().trim().to_lowercase();
            let conf = config_clone.lock().unwrap();
            let mut pid_lock = sub_pid_clone.lock().unwrap();

            match cmd.as_str() {
                "space" => {
                    if let Some(pid) = *pid_lock {
                        conf.bring_to_front(pid);
                    } else if let Ok(new_pid) = conf.spawn_detached() {
                        *pid_lock = Some(new_pid);
                    }
                }
                "k" => {
                    if let Some(pid) = *pid_lock {
                        let _ = conf.kill_sub_instance_by_pid(pid);
                        *pid_lock = None;
                    }
                }
                "c" => {
                    let _ = conf.clear_token();
                }
                "o" => {
                    let _ = conf.open_sandbox_in_explorer();
                }
                "q" => {
                    std::process::exit(0);
                }
                _ => {}
            }
        })
        .build(&window)?;

    // 5. 事件循环与托盘最小化调度
    let config_tray = Arc::clone(&config);
    let sub_pid_tray = Arc::clone(&sub_pid);

    let restore_window = |win: &tao::window::Window| {
        win.set_visible(true);
        win.set_minimized(false);
        win.set_focus();
        #[cfg(target_os = "windows")]
        {
            use tao::platform::windows::WindowExtWindows;
            use windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
            unsafe {
                SetForegroundWindow(win.hwnd() as _);
            }
        }
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        // 保持托盘实例和 webview 生命周期存活
        let _ = &_tray;
        let _ = &_webview;

        match event {
            Event::NewEvents(StartCause::Init) => {}
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                // 点击右上角 [X] 关闭按钮时，缩小/隐藏至系统托盘，不退出应用
                window.set_visible(false);
            }
            Event::UserEvent(UserEvent::Tray(
                tray_icon::TrayIconEvent::Click {
                    button: tray_icon::MouseButton::Left,
                    button_state: tray_icon::MouseButtonState::Up,
                    ..
                }
                | tray_icon::TrayIconEvent::DoubleClick {
                    button: tray_icon::MouseButton::Left,
                    ..
                },
            )) => {
                restore_window(&window);
            }
            Event::UserEvent(UserEvent::Tray(_)) => {}
            Event::UserEvent(UserEvent::Menu(menu_event)) => {
                if menu_event.id == show_id {
                    restore_window(&window);
                } else if menu_event.id == sub_id {
                    let conf = config_tray.lock().unwrap();
                    let mut pid_lock = sub_pid_tray.lock().unwrap();
                    if let Some(pid) = *pid_lock {
                        conf.bring_to_front(pid);
                    } else if let Ok(new_pid) = conf.spawn_detached() {
                        *pid_lock = Some(new_pid);
                    }
                } else if menu_event.id == quit_id {
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => (),
        }
    });
}
