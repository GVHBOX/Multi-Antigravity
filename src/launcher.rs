use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

const WIN32_DETACHED_FLAGS: u32 = 0x01000208;

pub struct LauncherConfig {
    pub slot: usize,
    pub executable_path: PathBuf,
    pub sandbox_root: PathBuf,
    pub home_dir: PathBuf,
    pub roaming_dir: PathBuf,
    pub token_path: PathBuf,
}

impl LauncherConfig {
    pub fn new() -> Result<Self> {
        Self::for_slot(1)
    }

    pub fn for_slot(slot: usize) -> Result<Self> {
        let base_dir = Self::resolve_base_dir()?;
        let instance_name = format!("instance_{}", slot + 1);
        let sandbox_root = base_dir.join("data").join(instance_name);
        let home_dir = sandbox_root.join("home");
        let roaming_dir = sandbox_root.join("AppData").join("Roaming");
        let token_path = home_dir.join(".gemini").join("jetski-standalone-oauth-token");

        let local_app_data = env::var("LOCALAPPDATA").context("LOCALAPPDATA is not set")?;
        let executable_path = PathBuf::from(local_app_data)
            .join("Programs")
            .join("antigravity")
            .join("Antigravity.exe");

        Ok(Self {
            slot,
            executable_path,
            sandbox_root,
            home_dir,
            roaming_dir,
            token_path,
        })
    }

    fn resolve_base_dir() -> Result<PathBuf> {
        if let Ok(exe_path) = env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                if exe_dir.join("data").exists() || exe_dir.join("Multi-Antigravity.exe").exists() || exe_dir.join("multi-antigravity.exe").exists() {
                    return Ok(exe_dir.to_path_buf());
                }
                if let Some(parent) = exe_dir.parent() {
                    if parent.join("data").exists() {
                        return Ok(parent.to_path_buf());
                    }
                    if let Some(grandparent) = parent.parent() {
                        if grandparent.join("data").exists() {
                            return Ok(grandparent.to_path_buf());
                        }
                    }
                }
                return Ok(exe_dir.to_path_buf());
            }
        }
        env::current_dir().context("Failed to get current directory")
    }

    pub fn ensure_sandbox_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.home_dir)
            .with_context(|| format!("Failed to create home dir: {:?}", self.home_dir))?;
        fs::create_dir_all(self.roaming_dir.join("Antigravity"))
            .with_context(|| format!("Failed to create roaming dir: {:?}", self.roaming_dir))?;

        let sandbox_config = self.home_dir.join(".gemini").join("config");
        let _ = fs::create_dir_all(&sandbox_config);

        if let Ok(real_user) = env::var("USERPROFILE") {
            let host_user = PathBuf::from(&real_user);

            let host_locallow = host_user.join("AppData").join("LocalLow");
            let sub_appdata = self.home_dir.join("AppData");
            let sub_locallow = sub_appdata.join("LocalLow");
            if host_locallow.exists() {
                let is_junc = sub_locallow
                    .symlink_metadata()
                    .map(|m| {
                        use std::os::windows::fs::MetadataExt;
                        m.file_attributes() & 0x400 != 0
                    })
                    .unwrap_or(false);
                if !is_junc {
                    if sub_locallow.exists() {
                        let _ = fs::remove_dir_all(&sub_locallow);
                    }
                    if !sub_locallow.exists() {
                        let _ = fs::create_dir_all(&sub_appdata);
                        let _ = Command::new("cmd")
                            .args(["/c", "mklink", "/J", &sub_locallow.to_string_lossy(), &host_locallow.to_string_lossy()])
                            .creation_flags(WIN32_DETACHED_FLAGS)
                            .output();
                    }
                }
            }

            let shared_dirs = ["skills", "rules", "plugins", "sidecars"];
            for dir_name in shared_dirs {
                let host_dir = host_user.join(".gemini").join("config").join(dir_name);
                let sub_dir = sandbox_config.join(dir_name);
                if host_dir.exists() && !sub_dir.exists() {
                    let _ = Command::new("cmd")
                        .args(["/c", "mklink", "/J", &sub_dir.to_string_lossy(), &host_dir.to_string_lossy()])
                        .creation_flags(WIN32_DETACHED_FLAGS)
                        .output();
                }
            }

            let host_mcp = host_user.join(".gemini").join("config").join("mcp_config.json");
            let sub_mcp = sandbox_config.join("mcp_config.json");
            if host_mcp.exists() {
                let should_copy = if sub_mcp.exists() {
                    fs::metadata(&sub_mcp).map(|m| m.len() == 0).unwrap_or(false)
                } else {
                    true
                };
                if should_copy {
                    let _ = fs::copy(&host_mcp, &sub_mcp);
                }
            }
        }

        Ok(())
    }

    pub fn is_token_present(&self) -> bool {
        self.token_path.exists()
    }

    pub fn clear_token(&self) -> Result<bool> {
        if self.token_path.exists() {
            fs::remove_file(&self.token_path)
                .with_context(|| format!("Failed to delete token at {:?}", self.token_path))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn open_sandbox_in_explorer(&self) -> Result<()> {
        self.ensure_sandbox_dirs()?;
        Command::new("explorer.exe")
            .arg(&self.sandbox_root)
            .spawn()
            .context("Failed to launch explorer.exe")?;
        Ok(())
    }

    pub fn spawn_detached(&self) -> Result<u32> {
        self.ensure_sandbox_dirs()?;

        if !self.executable_path.exists() {
            anyhow::bail!(
                "Antigravity.exe not found at {:?}. Please verify installation.",
                self.executable_path
            );
        }

        let user_data_dir = self.roaming_dir.join("Antigravity");
        let user_data_arg = format!("--user-data-dir={}", user_data_dir.display());

        let mut cmd = Command::new(&self.executable_path);
        cmd.arg(&user_data_arg);

        cmd.env("USERPROFILE", &self.home_dir);
        cmd.env("HOME", &self.home_dir);
        cmd.env("APPDATA", &self.roaming_dir);

        let ssh_port = 50000 + self.slot as u32;
        cmd.env("SSH_CONNECTION", format!("127.0.0.1 {} 127.0.0.1 22", ssh_port));

        if let Ok(real_local) = env::var("LOCALAPPDATA") {
            cmd.env("LOCALAPPDATA", real_local);
        }

        cmd.creation_flags(WIN32_DETACHED_FLAGS);

        let child = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn {:?}", self.executable_path))?;

        Ok(child.id())
    }

    pub fn kill_sub_instance_by_pid(&self, pid: u32) -> Result<()> {

        let output = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(WIN32_DETACHED_FLAGS)
            .output()
            .with_context(|| format!("Failed to execute taskkill for PID {}", pid))?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = format!("{} {}", stderr, stdout);

        if combined.contains("not found")
            || combined.contains("找不到")
            || combined.contains("没有找到")
        {
            return Ok(());
        }

        anyhow::bail!(
            "终止进程失败 (PID: {}): {}",
            pid,
            stderr.trim()
        )
    }

    pub fn spawn_host_native() -> Result<u32> {
        let local_app_data = env::var("LOCALAPPDATA").context("LOCALAPPDATA is not set")?;
        let executable_path = PathBuf::from(local_app_data)
            .join("Programs")
            .join("antigravity")
            .join("Antigravity.exe");

        if !executable_path.exists() {
            anyhow::bail!(
                "Antigravity.exe not found at {:?}. Please verify installation.",
                executable_path
            );
        }

        let mut cmd = Command::new(&executable_path);
        cmd.creation_flags(WIN32_DETACHED_FLAGS);

        let child = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn host {:?}", executable_path))?;

        Ok(child.id())
    }

    pub fn bring_to_front(&self, target_pid: u32) -> bool {
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
            use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                BringWindowToTop, EnumWindows, GetForegroundWindow,
                GetWindowRect, GetWindowTextLengthW, GetWindowThreadProcessId, IsIconic,
                IsWindowVisible, SetForegroundWindow, ShowWindow, SW_RESTORE,
            };

            struct SearchState {
                target_pid: u32,
                best_hwnd: HWND,
                found_main: bool,
            }

            unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> i32 {
                unsafe {
                    let state = &mut *(lparam as *mut SearchState);
                    let mut process_id: u32 = 0;
                    GetWindowThreadProcessId(hwnd, &mut process_id);

                    if process_id == state.target_pid && (IsWindowVisible(hwnd) != 0 || IsIconic(hwnd) != 0) {
                        let mut rect: RECT = std::mem::zeroed();
                        GetWindowRect(hwnd, &mut rect);
                        let w = rect.right - rect.left;
                        let h = rect.bottom - rect.top;
                        let title_len = GetWindowTextLengthW(hwnd);

                        if w > 100 && h > 100 && title_len > 0 {
                            state.best_hwnd = hwnd;
                            state.found_main = true;
                            return 0;
                        } else if state.best_hwnd.is_null() {
                            state.best_hwnd = hwnd;
                        }
                    }
                    1
                }
            }

            let mut state = SearchState {
                target_pid,
                best_hwnd: std::ptr::null_mut(),
                found_main: false,
            };

            EnumWindows(Some(enum_windows_callback), &mut state as *mut _ as LPARAM);

            if !state.best_hwnd.is_null() {
                let fg_hwnd = GetForegroundWindow();
                let fg_thread = if !fg_hwnd.is_null() {
                    GetWindowThreadProcessId(fg_hwnd, std::ptr::null_mut())
                } else {
                    0
                };
                let cur_thread = GetCurrentThreadId();

                if fg_thread != 0 && fg_thread != cur_thread {
                    AttachThreadInput(cur_thread, fg_thread, 1);
                    ShowWindow(state.best_hwnd, SW_RESTORE);
                    SetForegroundWindow(state.best_hwnd);
                    BringWindowToTop(state.best_hwnd);
                    AttachThreadInput(cur_thread, fg_thread, 0);
                } else {
                    ShowWindow(state.best_hwnd, SW_RESTORE);
                    SetForegroundWindow(state.best_hwnd);
                    BringWindowToTop(state.best_hwnd);
                }
                return true;
            }
        }
        false
    }
}
