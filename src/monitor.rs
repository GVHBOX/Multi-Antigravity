use sysinfo::{ProcessesToUpdate, System};

#[derive(Clone)]
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
    pub subs: [ProcessStats; 3],
    pub total_memory_mb: f64,
    pub system_total_ram_mb: f64,
    cached_host_ls_pid: Option<u32>,
    cached_host_port: Option<u16>,
    host_port_retry_count: u32,
    cached_sub_ls_pids: [Option<u32>; 3],
    cached_sub_ports: [Option<u16>; 3],
    sub_port_retry_counts: [u32; 3],
}

impl TelemetryMonitor {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_memory();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        let total_ram = (sys.total_memory() as f64) / (1024.0 * 1024.0);
        let system_total_ram_mb = if total_ram > 0.0 { total_ram } else { 32768.0 };
        Self {
            sys,
            host: ProcessStats::default(),
            sub: ProcessStats::default(),
            subs: [ProcessStats::default(), ProcessStats::default(), ProcessStats::default()],
            total_memory_mb: 0.0,
            system_total_ram_mb,
            cached_host_ls_pid: None,
            cached_host_port: None,
            host_port_retry_count: 0,
            cached_sub_ls_pids: [None; 3],
            cached_sub_ports: [None; 3],
            sub_port_retry_counts: [0; 3],
        }
    }

    pub fn refresh_single(&mut self, known_sub_pid: Option<u32>) {
        self.refresh([known_sub_pid, None, None]);
    }

    pub fn refresh(&mut self, known_sub_pids: [Option<u32>; 3]) {
        self.sys.refresh_memory();
        self.sys.refresh_processes(ProcessesToUpdate::All, true);

        let mut host_electron_pid = None;
        let mut host_ls_pid = None;
        let mut host_cpu = 0.0;
        let mut host_mem_bytes = 0u64;

        let mut sub_electron_pids = [None, None, None];
        let mut sub_ls_pids = [None, None, None];
        let mut sub_cpus = [0.0f32, 0.0f32, 0.0f32];
        let mut sub_mem_bytes = [0u64, 0u64, 0u64];

        let self_pid = std::process::id();

        for (pid, proc) in self.sys.processes() {
            let pid_u32 = pid.as_u32();
            if pid_u32 == self_pid {
                continue;
            }
            let name = proc.name().to_string_lossy().to_lowercase();
            let is_antigravity = name.contains("antigravity") && !name.contains("multi") && !name.contains("cockpit");
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

            let mut matched_slot = None;
            for slot in 0..3 {
                let tag = format!("instance_{}", slot + 2);
                if cmd_line.contains(&tag)
                    || known_sub_pids[slot] == Some(pid_u32)
                    || (proc.parent().map(|p| p.as_u32()) == known_sub_pids[slot] && known_sub_pids[slot].is_some())
                {
                    matched_slot = Some(slot);
                    break;
                }
            }

            let is_main_electron = is_antigravity && !cmd_line.contains("--type=");

            if let Some(slot) = matched_slot {
                if is_main_electron || (is_antigravity && sub_electron_pids[slot].is_none()) {
                    sub_electron_pids[slot] = Some(pid_u32);
                } else if is_ls && sub_ls_pids[slot].is_none() {
                    sub_ls_pids[slot] = Some(pid_u32);
                }
                sub_cpus[slot] += proc.cpu_usage();
                sub_mem_bytes[slot] += proc.memory();
            } else {
                if is_main_electron || (is_antigravity && host_electron_pid.is_none()) {
                    host_electron_pid = Some(pid_u32);
                } else if is_ls && host_ls_pid.is_none() {
                    host_ls_pid = Some(pid_u32);
                }
                host_cpu += proc.cpu_usage();
                host_mem_bytes += proc.memory();
            }
        }

        if host_ls_pid != self.cached_host_ls_pid {
            self.cached_host_ls_pid = host_ls_pid;
            self.cached_host_port = None;
            self.host_port_retry_count = 0;
        }

        if let Some(pid) = host_ls_pid {
            if self.cached_host_port.is_none() {
                self.host_port_retry_count = self.host_port_retry_count.wrapping_add(1);
                if self.host_port_retry_count <= 10 || self.host_port_retry_count % 5 == 0 {
                    self.cached_host_port = detect_listening_port(pid);
                }
            }
        } else {
            self.cached_host_port = None;
            self.host_port_retry_count = 0;
        }

        for slot in 0..3 {
            let ls_pid = sub_ls_pids[slot];
            if ls_pid != self.cached_sub_ls_pids[slot] {
                self.cached_sub_ls_pids[slot] = ls_pid;
                self.cached_sub_ports[slot] = None;
                self.sub_port_retry_counts[slot] = 0;
            }

            if let Some(pid) = ls_pid {
                if self.cached_sub_ports[slot].is_none() {
                    self.sub_port_retry_counts[slot] = self.sub_port_retry_counts[slot].wrapping_add(1);
                    if self.sub_port_retry_counts[slot] <= 10 || self.sub_port_retry_counts[slot] % 5 == 0 {
                        self.cached_sub_ports[slot] = detect_listening_port(pid);
                    }
                }
            } else {
                self.cached_sub_ports[slot] = None;
                self.sub_port_retry_counts[slot] = 0;
            }
        }

        let num_cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .max(1) as f32;

        self.host = ProcessStats {
            is_running: host_electron_pid.is_some(),
            electron_pid: host_electron_pid,
            ls_pid: host_ls_pid,
            cpu_usage: (host_cpu / num_cpus).min(100.0),
            memory_rss_mb: (host_mem_bytes as f64) / (1024.0 * 1024.0),
            ls_port: self.cached_host_port,
        };

        for slot in 0..3 {
            self.subs[slot] = ProcessStats {
                is_running: sub_electron_pids[slot].is_some(),
                electron_pid: sub_electron_pids[slot],
                ls_pid: sub_ls_pids[slot],
                cpu_usage: (sub_cpus[slot] / num_cpus).min(100.0),
                memory_rss_mb: (sub_mem_bytes[slot] as f64) / (1024.0 * 1024.0),
                ls_port: self.cached_sub_ports[slot],
            };
        }

        self.sub = self.subs[0].clone();
        self.total_memory_mb = self.host.memory_rss_mb
            + self.subs[0].memory_rss_mb
            + self.subs[1].memory_rss_mb
            + self.subs[2].memory_rss_mb;
        let total_ram = (self.sys.total_memory() as f64) / (1024.0 * 1024.0);
        if total_ram > 0.0 {
            self.system_total_ram_mb = total_ram;
        }
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
