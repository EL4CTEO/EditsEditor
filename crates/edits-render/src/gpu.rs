//! GPU device setup (headless or shared with a window).

use std::sync::Arc;

use crate::{RenderError, Result};

#[derive(Clone)]
pub struct GpuContext {
    pub instance: Option<wgpu::Instance>,
    pub adapter_info: wgpu::AdapterInfo,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub limits: wgpu::Limits,
}

#[derive(Clone, Debug, Default)]
pub struct GpuOptions {
    /// Prefer a low-power (integrated) GPU.
    pub low_power: bool,
    /// Force the software fallback adapter (WARP on Windows, lavapipe/llvmpipe on Linux).
    pub software: bool,
}

fn backends() -> wgpu::Backends {
    if let Ok(b) = std::env::var("WGPU_BACKEND") {
        return wgpu::Backends::from_comma_list(&b);
    }
    if cfg!(windows) {
        // DX12 first (best driver support on Windows), Vulkan as alternative.
        wgpu::Backends::DX12 | wgpu::Backends::VULKAN
    } else if cfg!(target_os = "macos") {
        wgpu::Backends::METAL
    } else {
        wgpu::Backends::VULKAN | wgpu::Backends::GL
    }
}

impl GpuContext {
    /// Create a headless device.
    pub fn new(opts: &GpuOptions) -> Result<GpuContext> {
        pollster::block_on(Self::new_async(opts))
    }

    pub async fn new_async(opts: &GpuOptions) -> Result<GpuContext> {
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle_from_env();
        desc.backends = backends();
        let instance = wgpu::Instance::new(desc);
        let power = if opts.low_power { wgpu::PowerPreference::LowPower } else { wgpu::PowerPreference::HighPerformance };
        let mut adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: power,
                force_fallback_adapter: opts.software,
                compatible_surface: None,
                apply_limit_buckets: false,
            })
            .await
            .ok();
        if adapter.is_none() && !opts.software {
            adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: power,
                    force_fallback_adapter: true,
                    compatible_surface: None,
                    apply_limit_buckets: false,
                })
                .await
                .ok();
        }
        let adapter = adapter.ok_or_else(|| RenderError::Gpu("no GPU adapter found (install GPU drivers)".into()))?;
        let info = adapter.get_info();
        let limits = wgpu::Limits {
            max_texture_dimension_2d: adapter.limits().max_texture_dimension_2d.min(16384),
            ..wgpu::Limits::downlevel_defaults()
        }
        .using_resolution(adapter.limits());
        let features = adapter.features() & wgpu::Features::FLOAT32_FILTERABLE;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("edits-device"),
                required_features: features,
                required_limits: limits.clone(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|e| RenderError::Gpu(e.to_string()))?;
        install_error_logger(&device);
        tracing::info!(adapter = %info.name, backend = ?info.backend, "GPU ready");
        Ok(GpuContext { instance: Some(instance), adapter_info: info, device, queue, limits })
    }

    /// Wrap an existing device (e.g. the viewer's egui-wgpu device) so frames render directly
    /// into textures the UI can display without readback.
    pub fn from_device(device: wgpu::Device, queue: wgpu::Queue, adapter_info: wgpu::AdapterInfo) -> GpuContext {
        let limits = device.limits();
        GpuContext { instance: None, adapter_info, device, queue, limits }
    }

    pub fn describe(&self) -> String {
        format!("{} ({:?}, {:?})", self.adapter_info.name, self.adapter_info.backend, self.adapter_info.device_type)
    }
}

fn install_error_logger(device: &wgpu::Device) {
    device.on_uncaptured_error(Arc::new(|e: wgpu::Error| {
        tracing::error!("wgpu error: {e}");
    }));
}
