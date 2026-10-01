#![windows_subsystem = "windows"]

mod app;
mod gui;
mod i18n;
mod launcher;
mod monitor;
mod ui;

use anyhow::Result;
use app::App;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::env;
use std::io::stdout;
use std::panic;
use std::time::{Duration, Instant};

fn setup_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let use_tui = args.iter().any(|a| a == "--tui" || a == "-t");

    if !use_tui {
        if env::var("SLINT_BACKEND").is_err() {
            unsafe {
                env::set_var("SLINT_BACKEND", "winit-software");
            }
        }
        return gui::run_gui();
    }

    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }

    setup_panic_hook();

    let mut app = App::new()?;

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(200);
    let refresh_interval = Duration::from_millis(1200);
    let mut last_refresh = Instant::now();

    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        if app.should_quit {
            break;
        }

        if event::poll(tick_rate)?
            && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char(' ') => {
                            app.launch_or_bring_to_front();
                        }
                        KeyCode::Char('k') | KeyCode::Char('K') => {
                            app.stop_sub_instance();
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            app.restart_engine();
                        }
                        KeyCode::Char('c') | KeyCode::Char('C') => {
                            app.clear_token();
                        }
                        KeyCode::Char('o') | KeyCode::Char('O') => {
                            app.open_sandbox_dir();
                        }
                        KeyCode::Char('t') | KeyCode::Char('T') => {
                            app.toggle_language();
                        }
                        KeyCode::Char('l') | KeyCode::Char('L') => {
                            app.clear_logs();
                        }
                        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                            app.should_quit = true;
                            break;
                        }
                        _ => {}
                    }
                }

        if last_refresh.elapsed() >= refresh_interval {
            app.refresh();
            last_refresh = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    println!();
    println!(" [Antigravity Cockpit] 监控座舱已安全退出。");
    println!(" 提示：分身通过 Win32 DETACHED 独立生成，正在系统后台继续平稳运行！");
    println!();

    Ok(())
}
