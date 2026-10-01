use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

// Win32 Process Creation Flags
// 0x00000008: DETACHED_PROCESS (creates a process with no console window / detached from caller)
// 0x01000000: CREATE_BREAKAWAY_FROM_JOB (allows the process to breakaway from any job object)
// 0x00000200: CREATE_NEW_PROCESS_GROUP (creates a new process group, immune to Ctrl+C/Ctrl+Break)
const WIN32_DETACHED_FLAGS: u32 = 0x01000208;

pub struct LauncherConfig {
    pub executable_path: PathBuf,
    pub sandbox_root: PathBuf,
    pub home_dir: PathBuf,
    pub roaming_dir: PathBuf,
    pub token_path: PathBuf,
}

impl LauncherConfig {
    pub fn new() -> Result<Self> {
        let base_dir = env::current_dir().context("Failed to get current directory")?;
        let sandbox_root = base_dir.join("data").join("instance_2");
        let home_dir = sandbox_root.join("home");
        let roaming_dir = sandbox_root.join("AppData").join("Roaming");
        let token_path = home_dir.join(".gemini").join("jetski-standalone-oauth-token");

        // Locate Antigravity.exe
        let local_app_data = env::var("LOCALAPPDATA").context("LOCALAPPDATA is not set")?;
        let executable_path = PathBuf::from(local_app_data)
            .join("Programs")
            .join("antigravity")
            .join("Antigravity.exe");

        Ok(Self {
            executable_path,
            sandbox_root,
            home_dir,
            roaming_dir,
            token_path,
        })
    }

    pub fn ensure_sandbox_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.home_dir)
            .with_context(|| format!("Failed to create home dir: {:?}", self.home_dir))?;
        fs::create_dir_all(self.roaming_dir.join("Antigravity"))
            .with_context(|| format!("Failed to create roaming dir: {:?}", self.roaming_dir))?;
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

        // Spawn detached process
        let mut cmd = Command::new(&self.executable_path);
        cmd.arg(&user_data_arg);

        // Environment isolation
        cmd.env("USERPROFILE", &self.home_dir);
        cmd.env("HOME", &self.home_dir);
        cmd.env("APPDATA", &self.roaming_dir);

        // SSH_CONNECTION bypass hook: forces Language Server to store tokens in files (.gemini)
        // instead of Windows Credential Manager, preventing any conflict with the host instance.
        cmd.env("SSH_CONNECTION", "127.0.0.1 50000 127.0.0.1 22");

        // CRITICAL: DO NOT override LOCALAPPDATA. Keeping the real LOCALAPPDATA allows
        // Chrome to open with the user's authentic local Chrome profile, enabling 1-click
        // account selection without losing browser sessions.
        if let Ok(real_local) = env::var("LOCALAPPDATA") {
            cmd.env("LOCALAPPDATA", real_local);
        }

        // Apply Win32 Detached flags
        cmd.creation_flags(WIN32_DETACHED_FLAGS);

        let child = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn {:?}", self.executable_path))?;

        Ok(child.id())
    }

    pub fn kill_sub_instance_by_pid(&self, pid: u32) -> Result<()> {
        // Use taskkill /T /F to kill the entire process tree (Electron + LanguageServer)
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(WIN32_DETACHED_FLAGS)
            .output();
        Ok(())
    }

    pub fn bring_to_front(&self, target_pid: u32) -> bool {
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::Foundation::{HWND, LPARAM};
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                EnumWindows, GetWindowThreadProcessId, IsWindowVisible, SetForegroundWindow,
                ShowWindow, SW_RESTORE,
            };

            struct SearchState {
                target_pid: u32,
                found_hwnd: HWND,
            }

            unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> i32 {
                unsafe {
                    let state = &mut *(lparam as *mut SearchState);
                    let mut process_id: u32 = 0;
                    GetWindowThreadProcessId(hwnd, &mut process_id);

                    if process_id == state.target_pid && IsWindowVisible(hwnd) != 0 {
                        state.found_hwnd = hwnd;
                        return 0; // stop enumerating
                    }
                    1 // continue
                }
            }

            let mut state = SearchState {
                target_pid,
                found_hwnd: std::ptr::null_mut(),
            };

            EnumWindows(Some(enum_windows_callback), &mut state as *mut _ as LPARAM);

            if !state.found_hwnd.is_null() {
                ShowWindow(state.found_hwnd, SW_RESTORE);
                SetForegroundWindow(state.found_hwnd);
                return true;
            }
        }
        false
    }
}
