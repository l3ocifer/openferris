use ferris_common::{GpuInfo, ResourceManifest};
use sysinfo::{Disks, System};

/// Detect local CPU, RAM, disk, and GPU resources.
pub fn detect() -> ResourceManifest {
    let sys = System::new_all();

    let cpu_cores = sys.cpus().len() as u16;
    let ram_mb = sys.total_memory() / (1024 * 1024);

    let disks = Disks::new_with_refreshed_list();
    let storage_avail_mb = disks
        .iter()
        .map(|d| d.available_space())
        .max()
        .unwrap_or(0)
        / (1024 * 1024);

    ResourceManifest {
        cpu_cores,
        ram_mb,
        storage_avail_mb,
        gpu: detect_gpu(),
    }
}

fn detect_gpu() -> Option<GpuInfo> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        // Apple Silicon uses unified memory — report total RAM as VRAM
        // since GPU and CPU share the same memory pool.
        let sys = System::new_all();
        let vram_mb = sys.total_memory() / (1024 * 1024);
        return Some(GpuInfo {
            name: "Apple Silicon".into(),
            vram_mb,
        });
    }

    #[allow(unreachable_code)]
    None
}
