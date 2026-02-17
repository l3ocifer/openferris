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
        ollama_models: vec![],
    }
}

fn detect_gpu() -> Option<GpuInfo> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return Some(GpuInfo {
            name: "Apple Silicon".into(),
            vram_mb: 0,
        });
    }

    #[allow(unreachable_code)]
    None
}
