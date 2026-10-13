//! Windows sampling for Sampler (RUNNER-WIN): the same Snapshot as on Linux, read through Win32.

use std::time::Instant;

use super::{gb, Sampler, Snapshot};
use crate::winsys;

impl Sampler {
    pub(super) fn sample_windows(&mut self) -> Snapshot {
        let now = winsys::system_times();
        let cpu = match (self.cpu_prev, now) {
            (Some((b0, t0)), Some((b1, t1))) if t1 > t0 => {
                Some((b1.saturating_sub(b0) as f64 / (t1 - t0) as f64).clamp(0.0, 1.0))
            }
            _ => None,
        };
        self.cpu_prev = now;
        self.last = Some(Instant::now());

        let (ram_used, ram_total) = winsys::memory();
        let (disk_used, disk_total) = winsys::disk_usage(&self.disk);

        Snapshot {
            cpu,
            cpu_count: winsys::cpu_count(),
            ram_used_gb: gb(ram_used),
            ram_total_gb: gb(ram_total),
            disk_used_gb: gb(disk_used),
            disk_total_gb: gb(disk_total),
            uptime_secs: winsys::uptime_secs(),
            load1: 0.0,
            os: "Windows".into(),
            gpus: Vec::new(),
            gaming: None,
        }
    }
}
