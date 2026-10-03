use anyhow::Result;
use std::env;
use std::fs;
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

pub const VERSION_DLL: &[u8] = include_bytes!("../assets/proxy/version.dll");
pub const CONFIG_JSON: &str = include_str!("../assets/proxy/config.json");
pub const DEFAULT_PROXY_PORT: u16 = 7890;

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ProxyStatus {
    pub app_installed: bool,
    pub app_deployed: bool,
    pub ide_installed: bool,
    pub ide_deployed: bool,
    pub port_online: bool,
    pub port: u16,
}

impl ProxyStatus {
    pub fn is_any_deployed(&self) -> bool {
        self.app_deployed || self.ide_deployed
    }

    pub fn is_fully_deployed(&self) -> bool {
        let app_ok = !self.app_installed || self.app_deployed;
        let ide_ok = !self.ide_installed || self.ide_deployed;
        app_ok && ide_ok && (self.app_deployed || self.ide_deployed)
    }

    #[allow(dead_code)]
    pub fn summary_text(&self) -> &'static str {
        if self.is_fully_deployed() {
            if self.port_online {
                "代理就绪 (SOCKS5 7890)"
            } else {
                "代理已部署 (7890未开启)"
            }
        } else if self.is_any_deployed() {
            "部分部署"
        } else {
            "未部署代理"
        }
    }
}

pub struct ProxyManager;

impl ProxyManager {
    pub fn app_dir() -> Option<PathBuf> {
        let local_app_data = env::var("LOCALAPPDATA").ok()?;
        let dir = PathBuf::from(local_app_data).join("Programs").join("antigravity");
        if dir.exists() {
            Some(dir)
        } else {
            None
        }
    }

    pub fn ide_dir() -> Option<PathBuf> {
        let local_app_data = env::var("LOCALAPPDATA").ok()?;
        let dir = PathBuf::from(local_app_data).join("Programs").join("Antigravity IDE");
        if dir.exists() {
            Some(dir)
        } else {
            None
        }
    }

    pub fn is_port_online(port: u16) -> bool {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok()
    }

    pub fn check_status() -> ProxyStatus {
        let app_dir = Self::app_dir();
        let app_installed = app_dir.is_some();
        let app_deployed = app_dir
            .as_ref()
            .map(|d| d.join("version.dll").exists())
            .unwrap_or(false);

        let ide_dir = Self::ide_dir();
        let ide_installed = ide_dir.is_some();
        let ide_deployed = ide_dir
            .as_ref()
            .map(|d| d.join("version.dll").exists())
            .unwrap_or(false);

        let port_online = Self::is_port_online(DEFAULT_PROXY_PORT);

        ProxyStatus {
            app_installed,
            app_deployed,
            ide_installed,
            ide_deployed,
            port_online,
            port: DEFAULT_PROXY_PORT,
        }
    }

    pub fn deploy_all() -> Result<String> {
        let mut deployed = Vec::new();

        if let Some(dir) = Self::app_dir() {
            fs::write(dir.join("version.dll"), VERSION_DLL)?;
            fs::write(dir.join("config.json"), CONFIG_JSON)?;
            deployed.push("桌面版");
        }

        if let Some(dir) = Self::ide_dir() {
            fs::write(dir.join("version.dll"), VERSION_DLL)?;
            fs::write(dir.join("config.json"), CONFIG_JSON)?;
            deployed.push("IDE版");
        }

        if deployed.is_empty() {
            anyhow::bail!("未找到 Antigravity 或 Antigravity IDE 安装目录！");
        }

        Ok(format!("代理已成功部署至: {}", deployed.join(" & ")))
    }

    pub fn remove_all() -> Result<String> {
        let mut removed = Vec::new();

        if let Some(dir) = Self::app_dir() {
            let _ = fs::remove_file(dir.join("version.dll"));
            let _ = fs::remove_file(dir.join("config.json"));
            removed.push("桌面版");
        }

        if let Some(dir) = Self::ide_dir() {
            let _ = fs::remove_file(dir.join("version.dll"));
            let _ = fs::remove_file(dir.join("config.json"));
            removed.push("IDE版");
        }

        Ok(format!("已从 {} 移除代理注入文件", removed.join(" & ")))
    }
}
