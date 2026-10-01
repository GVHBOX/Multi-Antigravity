use sysinfo::{ProcessesToUpdate, System};

pub struct ProcessStats {
    pub is_running: bool,
    pub electron_pid: Option<u32>,
    pub ls_pid: Option<u32>,
    pub cpu_usage: f32,
    pub memory_rss_mb: f64,
    pub ls_port: Option<u16>,
}

impl Default for ProcessStats {
    fn default() -> Self {
        Self {
            is_running: false,
            electron_pid: None,
            ls_pid: None,
            cpu_usage: 0.0,
            memory_rss_mb: 0.0,
            ls_port: None,
        }
    }
}

pub struct TelemetryMonitor {
    sys: System,
    pub host: ProcessStats,
    pub sub: ProcessStats,
    pub total_memory_mb: f64,
    cached_host_ls_pid: Option<u32>,
    cached_host_port: Option<u16>,
    host_port_probed: bool,
    cached_sub_ls_pid: Option<u32>,
    cached_sub_port: Option<u16>,
    sub_port_probed: bool,
}

impl TelemetryMonitor {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        Self {
            sys,
            host: ProcessStats::default(),
            sub: ProcessStats::default(),
            total_memory_mb: 0.0,
            cached_host_ls_pid: None,
            cached_host_port: None,
            host_port_probed: false,
            cached_sub_ls_pid: None,
            cached_sub_port: None,
            sub_port_probed: false,
        }
    }

    pub fn refresh(&mut self, known_sub_pid: Option<u32>) {
        self.sys.refresh_processes(ProcessesToUpdate::All, true);

        let mut host_electron_pid = None;
        let mut host_ls_pid = None;
        let mut host_cpu = 0.0;
        let mut host_mem_bytes = 0u64;

        let mut sub_electron_pid = None;
        let mut sub_ls_pid = None;
        let mut sub_cpu = 0.0;
        let mut sub_mem_bytes = 0u64;

        let self_pid = std::process::id();

        for (pid, proc) in self.sys.processes() {
            let pid_u32 = pid.as_u32();
            if pid_u32 == self_pid {
                continue;
            }
            let name = proc.name().to_string_lossy().to_lowercase();
            let is_antigravity = name.contains("antigravity") && !name.contains("cockpit");
            let is_ls = name.contains("language_server") || name.contains("jetski");

            if !is_antigravity && !is_ls {
                continue;
            }

            let cmd_line = proc
                .cmd()
                .iter()
                .map(|s| s.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ");

            // Determine whether this process belongs to sub-instance or host
            let is_sub = cmd_line.contains("instance_2")
                || known_sub_pid == Some(pid_u32)
                || (proc.parent().map(|p| p.as_u32()) == known_sub_pid && known_sub_pid.is_some());

            let is_main_electron = is_antigravity && !cmd_line.contains("--type=");

            if is_sub {
                if is_main_electron && sub_electron_pid.is_none() {
                    sub_electron_pid = Some(pid_u32);
                } else if is_antigravity && sub_electron_pid.is_none() {
                    // Fallback if cmd line was empty or unreadable
                    sub_electron_pid = Some(pid_u32);
                } else if is_ls && sub_ls_pid.is_none() {
                    sub_ls_pid = Some(pid_u32);
                }
                sub_cpu += proc.cpu_usage();
                sub_mem_bytes += proc.memory();
            } else {
                if is_main_electron && host_electron_pid.is_none() {
                    host_electron_pid = Some(pid_u32);
                } else if is_antigravity && host_electron_pid.is_none() {
                    // Fallback
                    host_electron_pid = Some(pid_u32);
                } else if is_ls && host_ls_pid.is_none() {
                    host_ls_pid = Some(pid_u32);
                }
                host_cpu += proc.cpu_usage();
                host_mem_bytes += proc.memory();
            }
        }

        // Update host port cache
        if host_ls_pid != self.cached_host_ls_pid || !self.host_port_probed {
            self.cached_host_ls_pid = host_ls_pid;
            self.host_port_probed = true;
            self.cached_host_port = host_ls_pid.and_then(detect_listening_port);
        }

        // Update sub port cache
        if sub_ls_pid != self.cached_sub_ls_pid || !self.sub_port_probed {
            self.cached_sub_ls_pid = sub_ls_pid;
            self.sub_port_probed = true;
            self.cached_sub_port = sub_ls_pid.and_then(detect_listening_port);
        }

        self.host = ProcessStats {
            is_running: host_electron_pid.is_some(),
            electron_pid: host_electron_pid,
            ls_pid: host_ls_pid,
            cpu_usage: host_cpu,
            memory_rss_mb: (host_mem_bytes as f64) / (1024.0 * 1024.0),
            ls_port: self.cached_host_port,
        };

        self.sub = ProcessStats {
            is_running: sub_electron_pid.is_some(),
            electron_pid: sub_electron_pid,
            ls_pid: sub_ls_pid,
            cpu_usage: sub_cpu,
            memory_rss_mb: (sub_mem_bytes as f64) / (1024.0 * 1024.0),
            ls_port: self.cached_sub_port,
        };

        self.total_memory_mb = self.host.memory_rss_mb + self.sub.memory_rss_mb;
    }
}

fn detect_listening_port(pid: u32) -> Option<u16> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    let output = Command::new("netstat")
        .args(["-ano", "-p", "tcp"])
        .creation_flags(0x08000000)
        .output()
        .ok()?;

    let text = String::from_utf8_lossy(&output.stdout);
    let target = pid.to_string();
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 || !parts[0].eq_ignore_ascii_case("tcp") {
            continue;
        }
        if !parts.iter().any(|p| p.eq_ignore_ascii_case("listening")) {
            continue;
        }
        if parts[parts.len() - 1] != target {
            continue;
        }
        if let Some(port) = parts[1]
            .rfind(':')
            .and_then(|pos| parts[1][pos + 1..].parse::<u16>().ok())
        {
            return Some(port);
        }
    }
    None
}
