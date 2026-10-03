//! The compositor: pipelines, texture pool, upload cache, uniform arena and the per-frame
//! command recording API used by the engine.
//!
//! Design notes (performance):
//! - All work for a frame is recorded into ONE command encoder and submitted once.
//! - Uniform data for every draw goes into a few large, reused buffers (256-byte aligned
//!   sub-allocations), uploaded with one `write_buffer` per chunk right before submit.
//! - Working textures (rgba16float, linear premultiplied) come from a pool and are recycled
//!   *within* a frame as soon as the engine releases them, keeping VRAM flat.
//! - Normal/Add compositing uses fixed-function blending (no extra pass, no ping-pong).
//! - Static uploads (images, text, shapes, LUTs) live in a byte-budgeted LRU on the GPU.
//! - Readbacks can be pipelined (submit frame N, render N+1, then collect N).

use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    num::NonZeroUsize,
    sync::{Arc, mpsc},
};

use bytemuck::{Pod, Zeroable};
use edits_core::{BlendMode, MatteMode};
use edits_fx::EffectDef;
use edits_media::Frame as MediaFrame;
use lru::LruCache;
use parking_lot::Mutex;

use crate::{RenderError, Result, gpu::GpuContext, transform::PlaceParams};

pub const WORK_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
pub const MASK_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;
pub const UPLOAD_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
pub const OUTPUT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

const ALIGN: u64 = 256;
const CHUNK: u64 = 1 << 20;
const UPLOAD_BUDGET: usize = 768 << 20;

const COMMON: &str = include_str!("shaders/common.wgsl");

fn preprocess(src: &str) -> String {
    src.replace("#include common", COMMON)
}

pub struct GpuTex {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
}

impl GpuTex {
    fn bytes(&self) -> usize {
        let bpp = match self.format {
            wgpu::TextureFormat::Rgba16Float => 8,
            wgpu::TextureFormat::R16Float => 2,
            _ => 4,
        };
        (self.width * self.height) as usize * bpp
    }
}

/// Per-frame texture handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Tex(u32);

/// Uniform globals shared by every effect pass (must match the WGSL prelude).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct Globals {
    pub resolution: [f32; 2],
    pub time: f32,
    pub local_time: f32,
    pub progress: f32,
    pub duration: f32,
    pub frame: f32,
    pub seed: f32,
    pub beat_time: f32,
    pub beat_index: f32,
    pub bpm: f32,
    pub audio_level: f32,
    pub audio_bands: [f32; 4],
    pub pass_data: [f32; 4],
    pub texel: [f32; 2],
    pub src_resolution: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct EffectUniforms {
    g: Globals,
    p: [[f32; 4]; 32],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PlaceUniforms {
    model: [[f32; 4]; 4],
    comp: [f32; 4],
    uv_rect: [f32; 4],
    misc: [f32; 4],
    flags: [f32; 4],
    tint: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vec4x2 {
    a: [f32; 4],
    b: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MaskUniforms {
    a: [f32; 4],
    b: [f32; 4],
    c: [f32; 4],
    d: [f32; 4],
    e: [f32; 4],
    pts: [[f32; 4]; 32],
}

/// One mask shape to rasterize (composition pixels, relative to the composition center).
#[derive(Clone, Debug)]
pub struct MaskParams {
    /// 0 rect, 1 ellipse, 2 polygon
    pub shape: u32,
    /// 0 add, 1 subtract, 2 intersect, 3 difference
    pub mode: u32,
    pub invert: bool,
    pub center: [f32; 2],
    pub size: [f32; 2],
    pub radius: f32,
    pub feather: f32,
    pub expansion: f32,
    pub opacity: f32,
    pub offset: [f32; 2],
    pub rotation: f32,
    pub scale: [f32; 2],
    pub points: Vec<[f32; 2]>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OutputMode {
    /// Straight (un-premultiplied) alpha — for encoding and PNGs.
    Straight,
    /// Premultiplied alpha — for UI display.
    Premultiplied,
    /// Composite over an opaque color.
    Over([f32; 4]),
    /// Over a gray checkerboard (preview of transparency).
    Checker,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum PipeKey {
    Place { additive: bool },
    Source { add: bool },
    Blend,
    Mask,
    Util { entry: &'static str, format: wgpu::TextureFormat },
    Effect { module: u64, entry: String },
}

struct Slot {
    tex: Arc<GpuTex>,
    pooled: bool,
    cleared: bool,
    straight: bool,
    released: bool,
}

/// Inputs for running an effect.
pub struct EffectCall<'a> {
    pub def: &'a EffectDef,
    /// Uniform slot values in `def.slot_params()` order.
    pub params: &'a [[f32; 4]],
    pub globals: Globals,
    /// Main input (or transition "from"). `None` for generators.
    pub input: Option<Tex>,
    /// Transition "to".
    pub input2: Option<Tex>,
    /// Texture param (image / LUT).
    pub extra: Option<Tex>,
    /// Output size.
    pub size: (u32, u32),
}

pub struct Renderer {
    pub gpu: GpuContext,
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    samplers: [wgpu::Sampler; 3],
    modules: HashMap<&'static str, wgpu::ShaderModule>,
    effect_modules: HashMap<u64, std::result::Result<Arc<wgpu::ShaderModule>, String>>,
    pipelines: HashMap<(PipeKey, wgpu::TextureFormat), Arc<wgpu::RenderPipeline>>,
    pool: HashMap<(u32, u32, wgpu::TextureFormat), Vec<Arc<GpuTex>>>,
    uploads: LruCache<u64, Arc<GpuTex>>,
    upload_bytes: usize,
    arena: Vec<wgpu::Buffer>,
    staging: Arc<Mutex<HashMap<u64, Vec<wgpu::Buffer>>>>,
    dummy: Arc<GpuTex>,
    white: Arc<GpuTex>,
    pub stats: RenderStats,
}

#[derive(Clone, Debug, Default)]
pub struct RenderStats {
    pub frames: u64,
    pub passes: u64,
    pub textures_created: u64,
    pub upload_hits: u64,
    pub upload_misses: u64,
}

fn hash_str(s: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

impl Renderer {
    pub fn new(gpu: GpuContext) -> Renderer {
        let device = &gpu.device;
        let tex_entry = |b: u32| wgpu::BindGroupLayoutEntry {
            binding: b,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let samp_entry = |b: u32| wgpu::BindGroupLayoutEntry {
            binding: b,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("edits-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                samp_entry(1),
                tex_entry(2),
                tex_entry(3),
                tex_entry(4),
                samp_entry(5),
                samp_entry(6),
                tex_entry(7),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("edits-pipeline-layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let sampler = |addr: wgpu::AddressMode, filter: wgpu::FilterMode| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: addr,
                address_mode_v: addr,
                address_mode_w: addr,
                mag_filter: filter,
                min_filter: filter,
                ..Default::default()
            })
        };
        let samplers = [
            sampler(wgpu::AddressMode::ClampToEdge, wgpu::FilterMode::Linear),
            sampler(wgpu::AddressMode::Repeat, wgpu::FilterMode::Linear),
            sampler(wgpu::AddressMode::ClampToEdge, wgpu::FilterMode::Nearest),
        ];
        let mut modules = HashMap::new();
        for (name, src) in [
            ("place", include_str!("shaders/place.wgsl")),
            ("composite", include_str!("shaders/composite.wgsl")),
            ("mask", include_str!("shaders/mask.wgsl")),
            ("util", include_str!("shaders/util.wgsl")),
        ] {
            let m = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(name),
                source: wgpu::ShaderSource::Wgsl(preprocess(src).into()),
            });
            modules.insert(name, m);
        }
        let mk = |w: u32, h: u32, px: [u8; 4]| -> Arc<GpuTex> {
            let t = create_tex(device, w, h, UPLOAD_FORMAT, "const");
            gpu.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &t.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &px.repeat((w * h) as usize),
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(h) },
                wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            );
            Arc::new(t)
        };
        let dummy = mk(1, 1, [0, 0, 0, 0]);
        let white = mk(1, 1, [255, 255, 255, 255]);
        Renderer {
            layout,
            pipeline_layout,
            samplers,
            modules,
            effect_modules: HashMap::new(),
            pipelines: HashMap::new(),
            pool: HashMap::new(),
            uploads: LruCache::unbounded(),
            upload_bytes: 0,
            arena: vec![],
            staging: Arc::new(Mutex::new(HashMap::new())),
            dummy,
            white,
            stats: RenderStats::default(),
            gpu,
        }
    }

    /// Compile (and cache) an effect's shader module, returning WGSL/validation errors.
    pub fn prepare_effect(&mut self, def: &EffectDef) -> Result<()> {
        self.effect_module(def).map(|_| ())
    }

    fn effect_module(&mut self, def: &EffectDef) -> Result<Arc<wgpu::ShaderModule>> {
        let key = hash_str(&def.source) ^ hash_str(&def.id);
        if let Some(r) = self.effect_modules.get(&key) {
            return r.clone().map_err(RenderError::Shader);
        }
        // naga validation first gives readable, line-accurate errors
        let res = match def.validate() {
            Err(e) => Err(e.to_string()),
            Ok(()) => {
                let scope = self.gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
                let m = self.gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(&def.id),
                    source: wgpu::ShaderSource::Wgsl(def.module_source().into()),
                });
                match pollster::block_on(scope.pop()) {
                    Some(e) => Err(e.to_string()),
                    None => Ok(Arc::new(m)),
                }
            }
        };
        self.effect_modules.insert(key, res.clone());
        res.map_err(RenderError::Shader)
    }

    fn pipeline(&mut self, key: PipeKey, format: wgpu::TextureFormat, effect: Option<&EffectDef>) -> Result<Arc<wgpu::RenderPipeline>> {
        if let Some(p) = self.pipelines.get(&(key.clone(), format)) {
            return Ok(p.clone());
        }
        let over = wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING;
        let add = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let effect_module;
        let (module, vs, fs, blend, strip): (&wgpu::ShaderModule, &str, String, Option<wgpu::BlendState>, bool) = match &key {
            PipeKey::Place { additive } => {
                (&self.modules["place"], "vs_place", "fs_place".into(), Some(if *additive { add } else { over }), true)
            }
            PipeKey::Source { add: a } => {
                (&self.modules["composite"], "vs_full", "fs_source".into(), Some(if *a { add } else { over }), false)
            }
            PipeKey::Blend => (&self.modules["composite"], "vs_full", "fs_blend".into(), None, false),
            PipeKey::Mask => (&self.modules["mask"], "vs_full", "fs_mask".into(), None, false),
            PipeKey::Util { entry, .. } => (&self.modules["util"], "vs_full", entry.to_string(), None, false),
            PipeKey::Effect { entry, .. } => {
                effect_module = self.effect_module(effect.expect("effect def"))?;
                (&*effect_module, "vs_main", EffectDef::fragment_name(entry), None, false)
            }
        };
        let scope = self.gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let p = self.gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&fs),
            layout: Some(&self.pipeline_layout),
            vertex: wgpu::VertexState { module, entry_point: Some(vs), compilation_options: Default::default(), buffers: &[] },
            primitive: wgpu::PrimitiveState {
                topology: if strip { wgpu::PrimitiveTopology::TriangleStrip } else { wgpu::PrimitiveTopology::TriangleList },
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module,
                entry_point: Some(&fs),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: wgpu::ColorWrites::ALL })],
            }),
            multiview_mask: None,
            cache: None,
        });
        if let Some(e) = pollster::block_on(scope.pop()) {
            return Err(RenderError::Shader(format!("pipeline {fs}: {e}")));
        }
        let p = Arc::new(p);
        self.pipelines.insert((key, format), p.clone());
        Ok(p)
    }

    /// Begin recording a frame.
    pub fn frame(&mut self) -> FrameCtx<'_> {
        let encoder = self.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("edits-frame") });
        self.stats.frames += 1;
        FrameCtx { r: self, encoder: Some(encoder), slots: vec![], free: HashMap::new(), uniforms: vec![], chunk_offsets: vec![] }
    }

    /// Drop cached uploads (e.g. after a project reload) and free pooled textures.
    pub fn trim(&mut self) {
        self.uploads.clear();
        self.upload_bytes = 0;
        self.pool.clear();
    }

    pub fn upload_cache_bytes(&self) -> usize {
        self.upload_bytes
    }
}

fn create_tex(device: &wgpu::Device, w: u32, h: u32, format: wgpu::TextureFormat, label: &str) -> GpuTex {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    GpuTex { texture, view, width: w.max(1), height: h.max(1), format }
}

/// A frame being recorded.
pub struct FrameCtx<'r> {
    r: &'r mut Renderer,
    encoder: Option<wgpu::CommandEncoder>,
    slots: Vec<Slot>,
    free: HashMap<(u32, u32, wgpu::TextureFormat), Vec<Arc<GpuTex>>>,
    /// CPU copies of uniform chunks for this frame.
    uniforms: Vec<Vec<u8>>,
    chunk_offsets: Vec<u64>,
}

impl<'r> FrameCtx<'r> {
    pub fn renderer(&mut self) -> &mut Renderer {
        self.r
    }

    pub fn size(&self, t: Tex) -> (u32, u32) {
        let s = &self.slots[t.0 as usize];
        (s.tex.width, s.tex.height)
    }

    fn alloc(&mut self, w: u32, h: u32, format: wgpu::TextureFormat) -> Tex {
        let key = (w.max(1), h.max(1), format);
        let tex = self.free.get_mut(&key).and_then(|v| v.pop()).or_else(|| self.r.pool.get_mut(&key).and_then(|v| v.pop())).unwrap_or_else(
            || {
                self.r.stats.textures_created += 1;
                Arc::new(create_tex(&self.r.gpu.device, key.0, key.1, format, "work"))
            },
        );
        self.slots.push(Slot { tex, pooled: true, cleared: false, straight: false, released: false });
        Tex(self.slots.len() as u32 - 1)
    }

    fn wrap(&mut self, tex: Arc<GpuTex>, straight: bool) -> Tex {
        self.slots.push(Slot { tex, pooled: false, cleared: true, straight, released: false });
        Tex(self.slots.len() as u32 - 1)
    }

    /// New transparent working texture.
    pub fn target(&mut self, w: u32, h: u32) -> Tex {
        self.alloc(w, h, WORK_FORMAT)
    }

    /// Return a texture for reuse later in this frame.
    pub fn release(&mut self, t: Tex) {
        let s = &mut self.slots[t.0 as usize];
        if s.released || !s.pooled {
            return;
        }
        s.released = true;
        let key = (s.tex.width, s.tex.height, s.tex.format);
        self.free.entry(key).or_default().push(s.tex.clone());
    }

    fn push_uniform(&mut self, bytes: &[u8]) -> (usize, u64) {
        let len = bytes.len() as u64;
        let need = len.div_ceil(ALIGN) * ALIGN;
        let mut ci = self.uniforms.len().saturating_sub(1);
        if self.uniforms.is_empty() || self.chunk_offsets[ci] + need > CHUNK {
            self.uniforms.push(vec![0u8; CHUNK as usize]);
            self.chunk_offsets.push(0);
            ci = self.uniforms.len() - 1;
            while self.r.arena.len() <= ci {
                let b = self.r.gpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("edits-uniforms"),
                    size: CHUNK,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                self.r.arena.push(b);
            }
        }
        let off = self.chunk_offsets[ci];
        self.uniforms[ci][off as usize..(off + len) as usize].copy_from_slice(bytes);
        self.chunk_offsets[ci] = off + need;
        (ci, off)
    }

    fn view(&self, t: Option<Tex>, fallback_white: bool) -> &wgpu::TextureView {
        match t {
            Some(t) => &self.slots[t.0 as usize].tex.view,
            None => {
                if fallback_white {
                    &self.r.white.view
                } else {
                    &self.r.dummy.view
                }
            }
        }
    }

    fn bind_group(&self, uniform: (usize, u64, u64), tex: [Option<Tex>; 4]) -> wgpu::BindGroup {
        let (ci, off, size) = uniform;
        self.r.gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.r.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.r.arena[ci],
                        offset: off,
                        size: std::num::NonZeroU64::new(size),
                    }),
                },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.r.samplers[0]) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(self.view(tex[0], false)) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(self.view(tex[1], false)) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(self.view(tex[2], false)) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(&self.r.samplers[1]) },
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::Sampler(&self.r.samplers[2]) },
                wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::TextureView(self.view(tex[3], false)) },
            ],
        })
    }

    /// Record a pass. `draws` = list of (uniform bytes, textures); all drawn into `target`.
    fn run(
        &mut self,
        pipe: &wgpu::RenderPipeline,
        target: Tex,
        draws: &[(Vec<u8>, [Option<Tex>; 4])],
        vertices: u32,
        clear: Option<wgpu::Color>,
    ) {
        let mut groups = Vec::with_capacity(draws.len());
        for (bytes, tex) in draws {
            let (ci, off) = self.push_uniform(bytes);
            groups.push(self.bind_group((ci, off, bytes.len() as u64), *tex));
        }
        let slot = &mut self.slots[target.0 as usize];
        let load = match clear {
            Some(c) => wgpu::LoadOp::Clear(c),
            None if !slot.cleared => wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            None => wgpu::LoadOp::Load,
        };
        slot.cleared = true;
        let view = slot.tex.view.clone();
        let enc = self.encoder.as_mut().unwrap();
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(pipe);
        for g in &groups {
            pass.set_bind_group(0, g, &[]);
            pass.draw(0..vertices, 0..1);
        }
        drop(pass);
        self.r.stats.passes += 1;
    }

    /// Fill a texture with a solid (linear premultiplied) color.
    pub fn clear(&mut self, t: Tex, color: [f32; 4]) {
        let pipe = self.r.pipeline(PipeKey::Util { entry: "fs_fill", format: WORK_FORMAT }, WORK_FORMAT, None).unwrap();
        let u = Vec4x2 { a: [0.0; 4], b: color };
        self.run(&pipe, t, &[(bytemuck::bytes_of(&u).to_vec(), [None; 4])], 3, Some(wgpu::Color::TRANSPARENT));
    }

    /// Upload a decoded frame. With a `key`, the texture is cached across frames.
    pub fn upload(&mut self, key: Option<u64>, frame: &MediaFrame) -> Tex {
        if let Some(k) = key
            && let Some(t) = self.r.uploads.get(&k).cloned()
        {
            self.r.stats.upload_hits += 1;
            return self.wrap(t, true);
        }
        self.r.stats.upload_misses += 1;
        let data: std::borrow::Cow<[u8]> =
            if frame.premultiplied { std::borrow::Cow::Owned(unpremultiply(&frame.data)) } else { std::borrow::Cow::Borrowed(&frame.data) };
        let tex = match key {
            Some(_) => Arc::new(create_tex(&self.r.gpu.device, frame.width, frame.height, UPLOAD_FORMAT, "upload")),
            None => {
                let t = self.alloc(frame.width, frame.height, UPLOAD_FORMAT);
                self.slots[t.0 as usize].straight = true;
                self.slots[t.0 as usize].cleared = true;
                let tex = self.slots[t.0 as usize].tex.clone();
                write_rgba8(&self.r.gpu.queue, &tex, &data);
                return t;
            }
        };
        write_rgba8(&self.r.gpu.queue, &tex, &data);
        if let Some(k) = key {
            self.r.upload_bytes += tex.bytes();
            self.r.uploads.put(k, tex.clone());
            while self.r.upload_bytes > UPLOAD_BUDGET {
                match self.r.uploads.pop_lru() {
                    Some((_, old)) => self.r.upload_bytes -= old.bytes(),
                    None => break,
                }
            }
        }
        self.wrap(tex, true)
    }

    /// Upload a float RGBA texture (LUTs). Cached by key.
    pub fn upload_f32(&mut self, key: u64, w: u32, h: u32, data: &[f32]) -> Tex {
        if let Some(t) = self.r.uploads.get(&key).cloned() {
            return self.wrap(t, false);
        }
        let tex = Arc::new(create_tex(&self.r.gpu.device, w, h, WORK_FORMAT, "lut"));
        let halfs: Vec<u16> = data.iter().map(|v| f32_to_f16(*v)).collect();
        self.r.gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&halfs),
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(8 * w), rows_per_image: Some(h) },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        self.r.upload_bytes += tex.bytes();
        self.r.uploads.put(key, tex.clone());
        self.wrap(tex, false)
    }

    /// Is an upload with this key already cached?
    pub fn has_upload(&mut self, key: u64) -> bool {
        self.r.uploads.contains(&key)
    }

    /// Wrap a cached upload without data (must exist: check `has_upload`).
    pub fn cached(&mut self, key: u64) -> Option<Tex> {
        let t = self.r.uploads.get(&key).cloned()?;
        Some(self.wrap(t, true))
    }

    /// Draw `src` into `dst` with one or more transforms (multiple = motion blur samples, which
    /// are accumulated additively with weights summing to 1).
    pub fn place(&mut self, dst: Tex, src: Tex, samples: &[PlaceParams]) -> Result<()> {
        if samples.is_empty() {
            return Ok(());
        }
        let additive = samples.len() > 1;
        let pipe = self.r.pipeline(PipeKey::Place { additive }, WORK_FORMAT, None)?;
        let straight = self.slots[src.0 as usize].straight;
        let w = 1.0 / samples.len() as f32;
        let draws: Vec<(Vec<u8>, [Option<Tex>; 4])> = samples
            .iter()
            .map(|p| {
                let u = PlaceUniforms {
                    model: p.model,
                    comp: [p.comp[0], p.comp[1], p.half[0], p.half[1]],
                    uv_rect: p.uv_rect,
                    misc: [p.quad_offset[0], p.quad_offset[1], p.perspective, p.weight * if additive { w } else { 1.0 }],
                    flags: [if straight { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0],
                    tint: p.tint,
                };
                (bytemuck::bytes_of(&u).to_vec(), [Some(src), None, None, None])
            })
            .collect();
        if additive {
            // accumulate into a temp, then composite over dst so existing content isn't brightened
            let (w, h) = self.size(dst);
            let tmp = self.target(w, h);
            self.run(&pipe, tmp, &draws, 4, None);
            let out = self.composite(dst, tmp, BlendMode::Normal, 1.0, None, None)?;
            debug_assert_eq!(out, dst);
            self.release(tmp);
        } else {
            self.run(&pipe, dst, &draws, 4, None);
        }
        Ok(())
    }

    /// Composite `src` onto `dst`. Returns the handle holding the result (may be `dst`).
    pub fn composite(
        &mut self,
        dst: Tex,
        src: Tex,
        mode: BlendMode,
        opacity: f32,
        mask: Option<Tex>,
        matte: Option<(Tex, MatteMode)>,
    ) -> Result<Tex> {
        let matte_mode = match matte.map(|m| m.1) {
            None => 0.0,
            Some(MatteMode::Alpha) => 1.0,
            Some(MatteMode::AlphaInverted) => 2.0,
            Some(MatteMode::Luma) => 3.0,
            Some(MatteMode::LumaInverted) => 4.0,
        };
        let u = Vec4x2 {
            a: [opacity, mode.index() as f32, if mask.is_some() { 1.0 } else { 0.0 }, matte_mode],
            b: [if matte.is_some() { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0],
        };
        let bytes = bytemuck::bytes_of(&u).to_vec();
        let matte_tex = matte.map(|m| m.0);
        match mode {
            BlendMode::Normal | BlendMode::Add => {
                let pipe = self.r.pipeline(PipeKey::Source { add: mode == BlendMode::Add }, WORK_FORMAT, None)?;
                self.run(&pipe, dst, &[(bytes, [Some(src), None, mask, matte_tex])], 3, None);
                Ok(dst)
            }
            _ => {
                let (w, h) = self.size(dst);
                let out = self.target(w, h);
                let pipe = self.r.pipeline(PipeKey::Blend, WORK_FORMAT, None)?;
                self.run(&pipe, out, &[(bytes, [Some(src), Some(dst), mask, matte_tex])], 3, None);
                self.release(dst);
                Ok(out)
            }
        }
    }

    /// Rasterize a mask shape, combining with `prev` (None = first mask).
    pub fn mask(&mut self, prev: Option<Tex>, size: (u32, u32), m: &MaskParams) -> Result<Tex> {
        let pipe = self.r.pipeline(PipeKey::Mask, MASK_FORMAT, None)?;
        let mut pts = [[0f32; 4]; 32];
        let n = m.points.len().min(64);
        for (i, p) in m.points.iter().take(64).enumerate() {
            let v = &mut pts[i / 2];
            if i % 2 == 0 {
                v[0] = p[0];
                v[1] = p[1];
            } else {
                v[2] = p[0];
                v[3] = p[1];
            }
        }
        let u = MaskUniforms {
            a: [m.shape as f32, m.mode as f32, if m.invert { 1.0 } else { 0.0 }, n as f32],
            b: [m.center[0], m.center[1], m.size[0], m.size[1]],
            c: [m.radius, m.feather, m.expansion, m.opacity],
            d: [m.offset[0], m.offset[1], m.rotation, if prev.is_none() { 1.0 } else { 0.0 }],
            e: [m.scale[0], m.scale[1], size.0 as f32, size.1 as f32],
            pts,
        };
        let out = self.alloc(size.0, size.1, MASK_FORMAT);
        self.run(&pipe, out, &[(bytemuck::bytes_of(&u).to_vec(), [prev, None, None, None])], 3, None);
        if let Some(p) = prev {
            self.release(p);
        }
        Ok(out)
    }

    /// Linear blend between two textures (effect `mix`). Returns a new texture.
    pub fn mix(&mut self, a: Tex, b: Tex, t: f32) -> Result<Tex> {
        let (w, h) = self.size(b);
        let out = self.target(w, h);
        let pipe = self.r.pipeline(PipeKey::Util { entry: "fs_mix", format: WORK_FORMAT }, WORK_FORMAT, None)?;
        let u = Vec4x2 { a: [t, 0.0, 0.0, 0.0], b: [0.0; 4] };
        self.run(&pipe, out, &[(bytemuck::bytes_of(&u).to_vec(), [Some(a), Some(b), None, None])], 3, None);
        Ok(out)
    }

    /// Resample/copy a texture into a new working texture (optionally scaled by `gain`).
    pub fn copy(&mut self, src: Tex, size: (u32, u32), gain: f32) -> Result<Tex> {
        let out = self.target(size.0, size.1);
        let entry = if self.slots[src.0 as usize].straight { "fs_premul" } else { "fs_copy" };
        let pipe = self.r.pipeline(PipeKey::Util { entry, format: WORK_FORMAT }, WORK_FORMAT, None)?;
        let u = Vec4x2 { a: [gain, 0.0, 0.0, 0.0], b: [0.0; 4] };
        self.run(&pipe, out, &[(bytemuck::bytes_of(&u).to_vec(), [Some(src), None, None, None])], 3, None);
        Ok(out)
    }

    /// Run an effect (all passes). Returns the output texture.
    pub fn effect(&mut self, call: EffectCall) -> Result<Tex> {
        let def = call.def;
        let (w, h) = call.size;
        let mut outputs: Vec<Tex> = vec![];
        let mut named: HashMap<String, Tex> = HashMap::new();
        let input = match call.input {
            Some(t) if self.slots[t.0 as usize].straight => Some(self.copy(t, self.size(t), 1.0)?),
            other => other,
        };
        let input2 = match call.input2 {
            Some(t) if self.slots[t.0 as usize].straight => Some(self.copy(t, self.size(t), 1.0)?),
            other => other,
        };
        let mut prev = input;
        let mut params = [[0f32; 4]; 32];
        for (i, p) in call.params.iter().take(32).enumerate() {
            params[i] = *p;
        }
        for (pi, pass) in def.passes.iter().enumerate() {
            let pw = ((w as f32 * pass.scale).round() as u32).max(1);
            let ph = ((h as f32 * pass.scale).round() as u32).max(1);
            let resolve = |name: &str, prev: Option<Tex>, outs: &Vec<Tex>, named: &HashMap<String, Tex>| -> Option<Tex> {
                match name {
                    "prev" | "" => prev,
                    "input" | "to" => input2.or(input),
                    "from" => input,
                    "none" => None,
                    "extra" => call.extra,
                    n => n.parse::<usize>().ok().and_then(|i| outs.get(i).copied()).or_else(|| named.get(n).copied()),
                }
            };
            for _ in 0..pass.repeat.max(1) {
                let t0 = resolve(pass.inputs.first().map(String::as_str).unwrap_or("prev"), prev, &outputs, &named);
                let t1 = resolve(pass.inputs.get(1).map(String::as_str).unwrap_or("input"), prev, &outputs, &named);
                let t2 = match pass.inputs.get(2) {
                    Some(n) => resolve(n, prev, &outputs, &named),
                    None => call.extra,
                };
                let mut g = call.globals;
                g.resolution = [pw as f32, ph as f32];
                g.texel = [1.0 / pw as f32, 1.0 / ph as f32];
                g.pass_data = pass.data;
                g.src_resolution = t0
                    .map(|t| {
                        let (a, b) = self.size(t);
                        [a as f32, b as f32]
                    })
                    .unwrap_or([pw as f32, ph as f32]);
                let u = EffectUniforms { g, p: params };
                let pipe = self.r.pipeline(
                    PipeKey::Effect { module: hash_str(&def.source) ^ hash_str(&def.id), entry: pass.entry.clone() },
                    WORK_FORMAT,
                    Some(def),
                )?;
                let out = self.target(pw, ph);
                self.run(&pipe, out, &[(bytemuck::bytes_of(&u).to_vec(), [t0, t1, t2, None])], 3, None);
                outputs.push(out);
                prev = Some(out);
            }
            if let Some(n) = &pass.name {
                named.insert(n.clone(), prev.unwrap());
            }
            let _ = pi;
        }
        let result = prev.ok_or_else(|| RenderError::Shader("effect produced no output".into()))?;
        for o in outputs {
            if o != result {
                self.release(o);
            }
        }
        if input != call.input
            && let Some(i) = input
        {
            self.release(i);
        }
        if input2 != call.input2
            && let Some(i) = input2
        {
            self.release(i);
        }
        Ok(result)
    }

    fn finish_encoder(&mut self) -> wgpu::CommandBuffer {
        for (ci, data) in self.uniforms.iter().enumerate() {
            let used = self.chunk_offsets[ci] as usize;
            if used > 0 {
                self.r.gpu.queue.write_buffer(&self.r.arena[ci], 0, &data[..used]);
            }
        }
        self.encoder.take().unwrap().finish()
    }

    fn output_pass(&mut self, src: Tex, target_view: &wgpu::TextureView, format: wgpu::TextureFormat, mode: OutputMode) -> Result<()> {
        let (entry, a, b) = match mode {
            OutputMode::Straight => ("fs_output", [0.0, 1.0, 0.0, 0.0], [0.0; 4]),
            OutputMode::Premultiplied => ("fs_output", [1.0, 1.0, 0.0, 0.0], [0.0; 4]),
            OutputMode::Over(c) => ("fs_output", [2.0, 1.0, 0.0, 0.0], c),
            OutputMode::Checker => {
                let (w, h) = self.size(src);
                ("fs_checker", [0.0; 4], [12.0, 0.0, w as f32, h as f32])
            }
        };
        let pipe = self.r.pipeline(PipeKey::Util { entry, format }, format, None)?;
        let u = Vec4x2 { a, b };
        let (ci, off) = self.push_uniform(bytemuck::bytes_of(&u));
        let bg = self.bind_group((ci, off, std::mem::size_of::<Vec4x2>() as u64), [Some(src), None, None, None]);
        let enc = self.encoder.as_mut().unwrap();
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("output"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipe);
        pass.set_bind_group(0, &bg, &[]);
        pass.draw(0..3, 0..1);
        Ok(())
    }

    /// Render `src` into an external texture view (viewer display) and submit.
    pub fn present_to(mut self, src: Tex, view: &wgpu::TextureView, format: wgpu::TextureFormat, mode: OutputMode) -> Result<()> {
        self.output_pass(src, view, format, mode)?;
        let cb = self.finish_encoder();
        self.r.gpu.queue.submit([cb]);
        Ok(())
    }

    /// Finish the frame and start reading `src` back as RGBA8 sRGB. Collect with `wait()`.
    pub fn read_async(mut self, src: Tex, mode: OutputMode) -> Result<PendingReadback> {
        let (w, h) = self.size(src);
        let out = Arc::new(create_out_tex(&self.r.gpu.device, w, h));
        self.output_pass(src, &out.view, OUTPUT_FORMAT, mode)?;
        let row = (w * 4) as u64;
        let padded = row.div_ceil(256) * 256;
        let size = padded * h as u64;
        let buffer = self.r.staging.lock().get_mut(&size).and_then(|v| v.pop()).unwrap_or_else(|| {
            self.r.gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("readback"),
                size,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });
        self.encoder.as_mut().unwrap().copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &out.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(padded as u32), rows_per_image: Some(h) },
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        let cb = self.finish_encoder();
        let index = self.r.gpu.queue.submit([cb]);
        let (tx, rx) = mpsc::channel();
        buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        Ok(PendingReadback {
            device: self.r.gpu.device.clone(),
            buffer: Some(buffer),
            rx,
            index,
            width: w,
            height: h,
            padded: padded as u32,
            size,
            staging: self.r.staging.clone(),
            _keep: out,
        })
    }

    /// Finish the frame and read `src` back synchronously.
    pub fn read(self, src: Tex, mode: OutputMode) -> Result<MediaFrame> {
        self.read_async(src, mode)?.wait()
    }

    /// Submit without reading anything back.
    pub fn submit(mut self) {
        let cb = self.finish_encoder();
        self.r.gpu.queue.submit([cb]);
    }
}

impl Drop for FrameCtx<'_> {
    fn drop(&mut self) {
        // an unfinished frame still has to flush uniforms consistently; just discard the encoder
        self.encoder.take();
        for s in self.slots.drain(..) {
            if s.pooled {
                let key = (s.tex.width, s.tex.height, s.tex.format);
                let v = self.r.pool.entry(key).or_default();
                if v.len() < 32 && !v.iter().any(|t| Arc::ptr_eq(t, &s.tex)) {
                    v.push(s.tex);
                }
            }
        }
        // also recycle free-list textures that were never re-wrapped
        for (key, list) in self.free.drain() {
            let v = self.r.pool.entry(key).or_default();
            for t in list {
                if v.len() < 32 && !v.iter().any(|x| Arc::ptr_eq(x, &t)) {
                    v.push(t);
                }
            }
        }
    }
}

fn create_out_tex(device: &wgpu::Device, w: u32, h: u32) -> GpuTex {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("output"),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: OUTPUT_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    GpuTex { texture, view, width: w, height: h, format: OUTPUT_FORMAT }
}

/// A readback in flight.
pub struct PendingReadback {
    device: wgpu::Device,
    buffer: Option<wgpu::Buffer>,
    rx: mpsc::Receiver<std::result::Result<(), wgpu::BufferAsyncError>>,
    index: wgpu::SubmissionIndex,
    pub width: u32,
    pub height: u32,
    padded: u32,
    size: u64,
    staging: Arc<Mutex<HashMap<u64, Vec<wgpu::Buffer>>>>,
    _keep: Arc<GpuTex>,
}

impl PendingReadback {
    /// Block until the pixels are available. Returns straight-alpha (or as requested) RGBA8 sRGB.
    pub fn wait(mut self) -> Result<MediaFrame> {
        let _ = self.device.poll(wgpu::PollType::Wait { submission_index: Some(self.index.clone()), timeout: None });
        self.rx
            .recv()
            .map_err(|_| RenderError::Gpu("readback channel closed".into()))?
            .map_err(|e| RenderError::Gpu(format!("map failed: {e}")))?;
        let buffer = self.buffer.take().unwrap();
        let mut data = Vec::with_capacity((self.width * self.height * 4) as usize);
        {
            let view = buffer.get_mapped_range(..).map_err(|e| RenderError::Gpu(format!("{e:?}")))?;
            let row = (self.width * 4) as usize;
            for y in 0..self.height as usize {
                let s = y * self.padded as usize;
                data.extend_from_slice(&view[s..s + row]);
            }
        }
        buffer.unmap();
        let mut st = self.staging.lock();
        let v = st.entry(self.size).or_default();
        if v.len() < 4 {
            v.push(buffer);
        }
        Ok(MediaFrame::new(self.width, self.height, data, false))
    }
}

fn write_rgba8(queue: &wgpu::Queue, tex: &GpuTex, data: &[u8]) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo { texture: &tex.texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        data,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * tex.width), rows_per_image: Some(tex.height) },
        wgpu::Extent3d { width: tex.width, height: tex.height, depth_or_array_layers: 1 },
    );
}

fn unpremultiply(data: &[u8]) -> Vec<u8> {
    let mut out = data.to_vec();
    for px in out.chunks_exact_mut(4) {
        let a = px[3] as u32;
        if a > 0 && a < 255 {
            for c in &mut px[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    out
}

/// IEEE f32 -> f16 bits (round to nearest).
fn f32_to_f16(v: f32) -> u16 {
    let x = v.to_bits();
    let sign = ((x >> 16) & 0x8000) as u16;
    let exp = ((x >> 23) & 0xff) as i32;
    let mant = x & 0x7f_ffff;
    if exp == 255 {
        return sign | 0x7c00 | if mant != 0 { 0x200 } else { 0 };
    }
    let e = exp - 127 + 15;
    if e >= 31 {
        return sign | 0x7c00;
    }
    if e <= 0 {
        if e < -10 {
            return sign;
        }
        let m = (mant | 0x80_0000) >> (1 - e);
        return sign | ((m + 0x1000) >> 13) as u16;
    }
    sign | (((e as u32) << 10) + ((mant + 0x1000) >> 13)) as u16
}

#[allow(dead_code)]
fn nz(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n.max(1)).unwrap()
}
