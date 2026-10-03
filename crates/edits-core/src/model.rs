//! The project document. Everything an edit consists of lives here and is serialized as JSON
//! (`*.edits.json`). The format is designed to be written by AI agents: generous defaults,
//! short forms for common things, and descriptive doc comments that end up in the JSON schema.
//!
//! Coordinate system: pixels, +x right, +y down. A clip `position` is the offset of the layer's
//! center from the composition center (so `[0, 0]` = centered). Times are seconds.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{color::Color, easing::Easing, math::Vec2, property::Property, value::Value};

pub const FORMAT_VERSION: u32 = 1;

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}
fn yes() -> bool {
    true
}
fn is_true(b: &bool) -> bool {
    *b
}
fn one_prop() -> Property<f64> {
    Property::Static(1.0)
}
fn is_one_prop(p: &Property<f64>) -> bool {
    matches!(p, Property::Static(v) if *v == 1.0)
}
fn zero_prop() -> Property<f64> {
    Property::Static(0.0)
}
fn is_zero_prop(p: &Property<f64>) -> bool {
    matches!(p, Property::Static(v) if *v == 0.0)
}
fn one_vec() -> Property<Vec2> {
    Property::Static(Vec2::ONE)
}
fn is_one_vec(p: &Property<Vec2>) -> bool {
    matches!(p, Property::Static(v) if *v == Vec2::ONE)
}
fn zero_vec() -> Property<Vec2> {
    Property::Static(Vec2::ZERO)
}
fn is_zero_vec(p: &Property<Vec2>) -> bool {
    matches!(p, Property::Static(v) if *v == Vec2::ZERO)
}

// ------------------------------------------------------------------------------------------------
// Project
// ------------------------------------------------------------------------------------------------

/// Root of an EditsEditor project file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Project {
    /// File format version.
    #[serde(default = "format_version")]
    pub version: u32,
    #[serde(default)]
    pub meta: ProjectMeta,
    /// Id of the composition that is rendered/exported.
    #[serde(default = "main_id")]
    pub root: String,
    /// Compositions by id. Compositions can be nested via `{"type": "comp"}` clip sources.
    pub compositions: IndexMap<String, Composition>,
    /// Imported media and resource files by id.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub assets: IndexMap<String, Asset>,
    /// Musical timing (beats, drops, sections) used by expressions, snapping and presets.
    #[serde(default, skip_serializing_if = "is_default")]
    pub timing: Timing,
    /// Project-defined custom effects (WGSL), same format as built-in effects.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub custom_effects: IndexMap<String, String>,
    /// Project-defined presets (Rhai scripts with a `//!` TOML header), same format as built-ins.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub custom_presets: IndexMap<String, String>,
    /// Global variables readable from expressions via `var("name")`.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub variables: IndexMap<String, Value>,
    /// Shared Rhai function library available to every expression and script.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub script_library: String,
    /// Counter for generated ids (`c12`, `fx7`, ...).
    #[serde(default)]
    pub id_counter: u64,
}

fn format_version() -> u32 {
    FORMAT_VERSION
}
fn main_id() -> String {
    "main".into()
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectMeta {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub author: String,
    /// Free-form notes (e.g. the agent's plan for the edit).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

impl Project {
    pub fn new(name: &str, width: u32, height: u32, fps: f64, duration: f64) -> Self {
        let mut compositions = IndexMap::new();
        compositions.insert("main".to_string(), Composition::new("Main", width, height, fps, duration));
        Project {
            version: FORMAT_VERSION,
            meta: ProjectMeta { name: name.to_string(), ..Default::default() },
            root: "main".into(),
            compositions,
            assets: IndexMap::new(),
            timing: Timing::default(),
            custom_effects: IndexMap::new(),
            custom_presets: IndexMap::new(),
            variables: IndexMap::new(),
            script_library: String::new(),
            id_counter: 0,
        }
    }

    /// Generate a new unique id with the given prefix.
    pub fn new_id(&mut self, prefix: &str) -> String {
        loop {
            self.id_counter += 1;
            let id = format!("{prefix}{}", self.id_counter);
            if !self.id_exists(&id) {
                return id;
            }
        }
    }

    /// Whether any object in the project uses this id.
    pub fn id_exists(&self, id: &str) -> bool {
        if self.compositions.contains_key(id) || self.assets.contains_key(id) {
            return true;
        }
        self.compositions.values().any(|c| {
            c.markers.iter().any(|m| m.id == id)
                || c.effects.iter().any(|e| e.id == id)
                || c.tracks.iter().any(|t| {
                    t.id == id
                        || t.effects.iter().any(|e| e.id == id)
                        || t.clips.iter().any(|cl| cl.id == id || cl.effects.iter().any(|e| e.id == id))
                })
        })
    }

    pub fn root_comp(&self) -> Option<&Composition> {
        self.compositions.get(&self.root)
    }

    pub fn root_comp_mut(&mut self) -> Option<&mut Composition> {
        self.compositions.get_mut(&self.root)
    }
}

// ------------------------------------------------------------------------------------------------
// Timing
// ------------------------------------------------------------------------------------------------

/// Musical structure of the edit. Filled by audio analysis (`analyze_audio` with apply) or by hand.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Timing {
    /// Asset id of the music track the timing was derived from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Offset added to all beat times (seconds), e.g. if the music clip starts later in the timeline.
    #[serde(default, skip_serializing_if = "is_default")]
    pub offset: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bpm: Option<f64>,
    /// Beat times (seconds, timeline time = value + offset).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beats: Vec<f64>,
    /// Downbeats (first beat of each bar).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub downbeats: Vec<f64>,
    /// Big energy hits / drops.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub drops: Vec<f64>,
    /// Strong onsets (snares, hits) that aren't necessarily on the beat grid.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accents: Vec<f64>,
    /// Song sections.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<Section>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Section {
    pub start: f64,
    pub end: f64,
    /// e.g. "intro", "build", "drop", "break", "outro"
    #[serde(default)]
    pub label: String,
    /// Relative energy 0..1.
    #[serde(default)]
    pub energy: f64,
}

impl Timing {
    pub fn beats_abs(&self) -> impl Iterator<Item = f64> + '_ {
        self.beats.iter().map(move |b| b + self.offset)
    }
}

// ------------------------------------------------------------------------------------------------
// Composition / Track
// ------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Composition {
    #[serde(default)]
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// Frames per second (e.g. 23.976, 24, 30, 60).
    pub fps: f64,
    /// Duration in seconds.
    pub duration: f64,
    #[serde(default = "black")]
    pub background: Color,
    /// Tracks, bottom to top: later tracks are composited over earlier ones.
    #[serde(default)]
    pub tracks: Vec<Track>,
    /// Effects applied to the final composited image of this composition.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<EffectInstance>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub markers: Vec<Marker>,
    /// Motion blur settings for clips that enable `motion_blur`.
    #[serde(default, skip_serializing_if = "is_default")]
    pub motion_blur: MotionBlur,
    /// Master audio gain (linear).
    #[serde(default = "one_prop", skip_serializing_if = "is_one_prop")]
    pub volume: Property<f64>,
}

fn black() -> Color {
    Color::BLACK
}

impl Composition {
    pub fn new(name: &str, width: u32, height: u32, fps: f64, duration: f64) -> Self {
        Composition {
            name: name.into(),
            width,
            height,
            fps,
            duration,
            background: Color::BLACK,
            tracks: vec![],
            effects: vec![],
            markers: vec![],
            motion_blur: MotionBlur::default(),
            volume: one_prop(),
        }
    }

    pub fn frame_count(&self) -> u64 {
        (self.duration * self.fps).round().max(0.0) as u64
    }

    pub fn frame_time(&self, frame: u64) -> f64 {
        frame as f64 / self.fps
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MotionBlur {
    /// Number of sub-frame samples (1 = off). 8-16 looks great for fast zooms/shakes.
    #[serde(default = "mb_samples")]
    pub samples: u32,
    /// Shutter angle in degrees (180 = film look, 360 = maximum smear).
    #[serde(default = "mb_angle")]
    pub shutter_angle: f64,
}

impl Default for MotionBlur {
    fn default() -> Self {
        MotionBlur { samples: mb_samples(), shutter_angle: mb_angle() }
    }
}
fn mb_samples() -> u32 {
    8
}
fn mb_angle() -> f64 {
    180.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Track {
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub enabled: bool,
    /// Mute this track's audio.
    #[serde(default, skip_serializing_if = "is_default")]
    pub muted: bool,
    /// Solo this track (audio + video): only soloed tracks render when any track is soloed.
    #[serde(default, skip_serializing_if = "is_default")]
    pub solo: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub locked: bool,
    #[serde(default = "one_prop", skip_serializing_if = "is_one_prop")]
    pub opacity: Property<f64>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub blend_mode: BlendMode,
    /// Track audio gain (linear).
    #[serde(default = "one_prop", skip_serializing_if = "is_one_prop")]
    pub volume: Property<f64>,
    /// Effects applied to this track's composited result.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<EffectInstance>,
    #[serde(default)]
    pub clips: Vec<Clip>,
}

impl Track {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Track {
            id: id.into(),
            name: name.into(),
            enabled: true,
            muted: false,
            solo: false,
            locked: false,
            opacity: one_prop(),
            blend_mode: BlendMode::Normal,
            volume: one_prop(),
            effects: vec![],
            clips: vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Marker {
    #[serde(default)]
    pub id: String,
    pub t: f64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub duration: f64,
}

// ------------------------------------------------------------------------------------------------
// Clip
// ------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Clip {
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// Timeline start (seconds).
    pub start: f64,
    /// Timeline duration (seconds).
    pub duration: f64,
    /// What this clip shows/plays.
    pub source: ClipSource,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub enabled: bool,
    /// Hidden clips don't render directly but can be used as mattes (`matte.clip`).
    #[serde(default, skip_serializing_if = "is_default")]
    pub hidden: bool,

    // ---- time mapping ----
    /// Source in-point (seconds into the media).
    #[serde(default, rename = "in", skip_serializing_if = "is_default")]
    pub source_in: f64,
    /// Playback speed (1 = normal, 0.25 = slow-mo, 3 = fast). Animatable: keyframing speed
    /// creates smooth velocity ramps — source time is the integral of speed.
    #[serde(default = "one_prop", skip_serializing_if = "is_one_prop")]
    pub speed: Property<f64>,
    /// Absolute time remap: maps clip-local time -> source time (seconds). Overrides `in`/`speed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_remap: Option<Property<f64>>,
    /// Play the source backwards (from the out-point to the in-point).
    #[serde(default, skip_serializing_if = "is_default")]
    pub reverse: bool,
    /// Freeze on a single source time (seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freeze: Option<f64>,
    /// What happens when the source runs out.
    #[serde(default, skip_serializing_if = "is_default")]
    pub end_behavior: EndBehavior,

    // ---- visual ----
    #[serde(default, skip_serializing_if = "is_default")]
    pub transform: Transform,
    /// How media is sized into the composition before the transform.
    #[serde(default, skip_serializing_if = "is_default")]
    pub fit: Fit,
    /// Crop in source pixels fractions: [left, top, right, bottom] in 0..1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<[f64; 4]>,
    #[serde(default = "one_prop", skip_serializing_if = "is_one_prop")]
    pub opacity: Property<f64>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub blend_mode: BlendMode,
    /// Effect stack, applied in order (in composition space, after the transform).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<EffectInstance>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub masks: Vec<Mask>,
    /// Use another clip's alpha/luma as this clip's matte.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matte: Option<Matte>,
    /// Transition from the previous clip on the same track, starting at this clip's start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_in: Option<Transition>,
    /// Enable sub-frame motion blur (settings on the composition).
    #[serde(default, skip_serializing_if = "is_default")]
    pub motion_blur: bool,
    /// Temporal echo / trails: re-renders the clip at earlier times and blends them in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub echo: Option<Echo>,

    // ---- audio ----
    #[serde(default, skip_serializing_if = "is_default")]
    pub audio: ClipAudio,

    // ---- organisation ----
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

impl Clip {
    pub fn new(id: impl Into<String>, start: f64, duration: f64, source: ClipSource) -> Self {
        Clip {
            id: id.into(),
            name: String::new(),
            start,
            duration,
            source,
            enabled: true,
            hidden: false,
            source_in: 0.0,
            speed: one_prop(),
            time_remap: None,
            reverse: false,
            freeze: None,
            end_behavior: EndBehavior::Hold,
            transform: Transform::default(),
            fit: Fit::Contain,
            crop: None,
            opacity: one_prop(),
            blend_mode: BlendMode::Normal,
            effects: vec![],
            masks: vec![],
            matte: None,
            transition_in: None,
            motion_blur: false,
            echo: None,
            audio: ClipAudio::default(),
            tags: vec![],
            notes: String::new(),
        }
    }

    pub fn end(&self) -> f64 {
        self.start + self.duration
    }

    /// Is the clip active at timeline time `t`?
    pub fn is_active(&self, t: f64) -> bool {
        self.enabled && t >= self.start && t < self.end()
    }

    /// Map clip-local time to source time (seconds), before end-behavior handling.
    /// `media_duration` is used for `reverse`.
    pub fn source_time(&self, local: f64, media_duration: Option<f64>) -> f64 {
        if let Some(f) = self.freeze {
            return f;
        }
        let raw =
            if let Some(remap) = &self.time_remap { remap.sample(local, &local) } else { self.source_in + self.integrate_speed(local) };
        if self.reverse {
            let total = self.source_in + self.integrate_speed(self.duration);
            let end = media_duration.map(|d| d.min(total)).unwrap_or(total);
            end - (raw - self.source_in)
        } else {
            raw
        }
    }

    /// Integral of speed from 0 to `local` (clip seconds) => source seconds elapsed.
    pub fn integrate_speed(&self, local: f64) -> f64 {
        match &self.speed {
            Property::Static(s) => s * local,
            p => {
                // Simpson integration over adaptive number of steps.
                if local <= 0.0 {
                    return 0.0;
                }
                let n = ((local * 60.0).ceil() as usize).clamp(8, 4096) & !1;
                let h = local / n as f64;
                let f = |x: f64| p.sample(x, &1.0);
                let mut sum = f(0.0) + f(local);
                for i in 1..n {
                    let w = if i % 2 == 1 { 4.0 } else { 2.0 };
                    sum += w * f(i as f64 * h);
                }
                sum * h / 3.0
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndBehavior {
    /// Hold the last frame.
    #[default]
    Hold,
    /// Loop the source.
    Loop,
    /// Bounce back and forth.
    PingPong,
    /// Render nothing once the source ends.
    Transparent,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Fit {
    /// Fit inside the composition, preserving aspect ratio.
    #[default]
    Contain,
    /// Fill the composition, cropping overflow.
    Cover,
    /// Stretch to the composition size.
    Stretch,
    /// Native pixel size.
    None,
    /// Fit to composition width.
    Width,
    /// Fit to composition height.
    Height,
}

/// What a clip renders.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClipSource {
    /// Any imported asset: video, image, animated image (gif/webp/apng), svg, image sequence, audio.
    Media {
        asset: String,
        /// Disable the asset's audio for this clip.
        #[serde(default, skip_serializing_if = "is_default")]
        no_audio: bool,
        /// Disable the asset's video for this clip (audio only).
        #[serde(default, skip_serializing_if = "is_default")]
        no_video: bool,
    },
    /// Solid color filling the composition (or `size`).
    Solid {
        color: Property<crate::color::Color>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        size: Option<Vec2>,
    },
    /// Rich text with optional per-character animation.
    Text(Box<TextSource>),
    /// Vector shape.
    Shape(Box<ShapeSource>),
    /// Procedural generator effect (noise, speed lines, particles, gradients, ...).
    Generator {
        effect: String,
        #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
        params: IndexMap<String, Property<Value>>,
    },
    /// Nested composition.
    Comp { comp: String },
    /// Applies this clip's effects to everything composited below it.
    Adjustment,
}

impl ClipSource {
    pub fn kind_name(&self) -> &'static str {
        match self {
            ClipSource::Media { .. } => "media",
            ClipSource::Solid { .. } => "solid",
            ClipSource::Text(_) => "text",
            ClipSource::Shape(_) => "shape",
            ClipSource::Generator { .. } => "generator",
            ClipSource::Comp { .. } => "comp",
            ClipSource::Adjustment => "adjustment",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Transform {
    /// Offset of the layer center from the composition center (pixels).
    #[serde(default = "zero_vec", skip_serializing_if = "is_zero_vec")]
    pub position: Property<Vec2>,
    /// Anchor point offset from the layer center (pixels, layer space). Rotation/scale pivot.
    #[serde(default = "zero_vec", skip_serializing_if = "is_zero_vec")]
    pub anchor: Property<Vec2>,
    /// Scale factor (1 = 100%). Number or [x, y].
    #[serde(default = "one_vec", skip_serializing_if = "is_one_vec")]
    pub scale: Property<Vec2>,
    /// Rotation in degrees (clockwise).
    #[serde(default = "zero_prop", skip_serializing_if = "is_zero_prop")]
    pub rotation: Property<f64>,
    /// 3D rotation around the X axis (degrees) — tilts toward/away from camera.
    #[serde(default = "zero_prop", skip_serializing_if = "is_zero_prop")]
    pub rotation_x: Property<f64>,
    /// 3D rotation around the Y axis (degrees).
    #[serde(default = "zero_prop", skip_serializing_if = "is_zero_prop")]
    pub rotation_y: Property<f64>,
    /// Depth in pixels (positive = further away = smaller).
    #[serde(default = "zero_prop", skip_serializing_if = "is_zero_prop")]
    pub z: Property<f64>,
    /// Skew in degrees along X.
    #[serde(default = "zero_prop", skip_serializing_if = "is_zero_prop")]
    pub skew: Property<f64>,
    /// Camera distance for perspective (pixels). Default ≈ 1.2 × composition width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub perspective: Option<f64>,
    /// Flip horizontally / vertically.
    #[serde(default, skip_serializing_if = "is_default")]
    pub flip_x: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub flip_y: bool,
}

impl Default for Transform {
    fn default() -> Self {
        Transform {
            position: zero_vec(),
            anchor: zero_vec(),
            scale: one_vec(),
            rotation: zero_prop(),
            rotation_x: zero_prop(),
            rotation_y: zero_prop(),
            z: zero_prop(),
            skew: zero_prop(),
            perspective: None,
            flip_x: false,
            flip_y: false,
        }
    }
}

/// Evaluated transform at one instant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransformState {
    pub position: Vec2,
    pub anchor: Vec2,
    pub scale: Vec2,
    pub rotation: f64,
    pub rotation_x: f64,
    pub rotation_y: f64,
    pub z: f64,
    pub skew: f64,
}

impl Default for TransformState {
    fn default() -> Self {
        TransformState {
            position: Vec2::ZERO,
            anchor: Vec2::ZERO,
            scale: Vec2::ONE,
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_y: 0.0,
            z: 0.0,
            skew: 0.0,
        }
    }
}

// ------------------------------------------------------------------------------------------------
// Effects, transitions, masks
// ------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EffectInstance {
    #[serde(default)]
    pub id: String,
    /// Effect id from the library (`effects_list`), or a project custom effect id.
    pub effect: String,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub enabled: bool,
    /// Parameter values by name. Missing params use the effect's defaults.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub params: IndexMap<String, Property<Value>>,
    /// Mix between the unprocessed (0) and processed (1) image.
    #[serde(default = "one_prop", skip_serializing_if = "is_one_prop")]
    pub mix: Property<f64>,
    /// Only apply within this clip-local time range [start, end] (seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<[f64; 2]>,
}

impl EffectInstance {
    pub fn new(id: impl Into<String>, effect: impl Into<String>) -> Self {
        EffectInstance { id: id.into(), effect: effect.into(), enabled: true, params: IndexMap::new(), mix: one_prop(), range: None }
    }

    pub fn with(mut self, name: &str, v: impl Into<Property<Value>>) -> Self {
        self.params.insert(name.to_string(), v.into());
        self
    }
}

impl From<f64> for Property<Value> {
    fn from(v: f64) -> Self {
        Property::Static(Value::Num(v))
    }
}
impl From<&str> for Property<Value> {
    fn from(v: &str) -> Self {
        Property::Static(Value::Str(v.into()))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Transition {
    /// Transition effect id (kind = "transition" in the library).
    pub effect: String,
    /// Duration in seconds, starting at the clip's start.
    pub duration: f64,
    /// Easing of the transition progress.
    #[serde(default, skip_serializing_if = "is_default")]
    pub ease: Easing,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub params: IndexMap<String, Property<Value>>,
    /// Center the transition on the cut (starts `duration/2` before the clip start). Requires the
    /// previous clip to overlap or have source handles.
    #[serde(default, skip_serializing_if = "is_default")]
    pub centered: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Mask {
    pub shape: MaskShape,
    #[serde(default, skip_serializing_if = "is_default")]
    pub mode: MaskMode,
    /// Feather (soft edge) in pixels.
    #[serde(default = "zero_prop", skip_serializing_if = "is_zero_prop")]
    pub feather: Property<f64>,
    /// Grow (+) or shrink (-) the mask in pixels.
    #[serde(default = "zero_prop", skip_serializing_if = "is_zero_prop")]
    pub expansion: Property<f64>,
    #[serde(default = "one_prop", skip_serializing_if = "is_one_prop")]
    pub opacity: Property<f64>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub invert: bool,
    /// Offset applied to the mask shape (pixels).
    #[serde(default = "zero_vec", skip_serializing_if = "is_zero_vec")]
    pub offset: Property<Vec2>,
    /// Rotation of the shape (degrees).
    #[serde(default = "zero_prop", skip_serializing_if = "is_zero_prop")]
    pub rotation: Property<f64>,
    /// Scale of the shape.
    #[serde(default = "one_vec", skip_serializing_if = "is_one_vec")]
    pub scale: Property<Vec2>,
}

/// Mask geometry in composition pixels relative to the composition center.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MaskShape {
    Rect {
        #[serde(default = "zero_vec")]
        center: Property<Vec2>,
        size: Property<Vec2>,
        #[serde(default = "zero_prop")]
        radius: Property<f64>,
    },
    Ellipse {
        #[serde(default = "zero_vec")]
        center: Property<Vec2>,
        size: Property<Vec2>,
    },
    /// Closed polygon (up to 64 points), composition pixels relative to center.
    Polygon { points: Vec<Vec2> },
    /// SVG path data (`M 0 0 C ...`), composition pixels relative to center. Flattened to a polygon.
    Path { d: String },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MaskMode {
    #[default]
    Add,
    Subtract,
    Intersect,
    Difference,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Matte {
    /// Clip id whose rendered layer is used as the matte (often a `hidden` clip).
    pub clip: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub mode: MatteMode,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MatteMode {
    #[default]
    Alpha,
    AlphaInverted,
    Luma,
    LumaInverted,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Echo {
    /// Number of echoes (1..32).
    #[serde(default = "echo_count")]
    pub count: u32,
    /// Seconds between echoes (negative = future frames).
    #[serde(default = "echo_interval")]
    pub interval: f64,
    /// Opacity multiplier per echo step (0..1).
    #[serde(default = "echo_decay")]
    pub decay: f64,
    /// How echoes combine with each other.
    #[serde(default = "echo_blend")]
    pub blend: BlendMode,
    /// Draw echoes behind (true) or over (false) the main frame.
    #[serde(default = "yes")]
    pub behind: bool,
}

fn echo_count() -> u32 {
    4
}
fn echo_interval() -> f64 {
    1.0 / 24.0
}
fn echo_decay() -> f64 {
    0.6
}
fn echo_blend() -> BlendMode {
    BlendMode::Add
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ClipAudio {
    /// Linear gain (1 = unchanged, 0 = silent, 2 ≈ +6 dB).
    #[serde(default = "one_prop", skip_serializing_if = "is_one_prop")]
    pub volume: Property<f64>,
    /// Stereo pan -1 (left) .. 1 (right).
    #[serde(default = "zero_prop", skip_serializing_if = "is_zero_prop")]
    pub pan: Property<f64>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub mute: bool,
    /// Fade-in length (seconds).
    #[serde(default, skip_serializing_if = "is_default")]
    pub fade_in: f64,
    /// Fade-out length (seconds).
    #[serde(default, skip_serializing_if = "is_default")]
    pub fade_out: f64,
    /// Audio effects (lowpass, highpass, echo, distortion, bitcrush, stutter...).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<AudioEffect>,
}

impl Default for ClipAudio {
    fn default() -> Self {
        ClipAudio { volume: one_prop(), pan: zero_prop(), mute: false, fade_in: 0.0, fade_out: 0.0, effects: vec![] }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AudioEffect {
    /// Low-pass filter (muffled "underwater" sound when cutoff is low). Animatable cutoff (Hz).
    Lowpass {
        cutoff: Property<f64>,
        #[serde(default = "q_default")]
        q: f64,
    },
    /// High-pass filter (thin "radio" sound).
    Highpass {
        cutoff: Property<f64>,
        #[serde(default = "q_default")]
        q: f64,
    },
    /// Feedback delay.
    Echo {
        #[serde(default = "delay_default")]
        delay: f64,
        #[serde(default = "feedback_default")]
        feedback: f64,
        #[serde(default = "mix_default")]
        mix: f64,
    },
    /// Soft-clip distortion.
    Distortion {
        #[serde(default = "drive_default")]
        drive: f64,
    },
    /// Bit depth / sample-rate reduction.
    Bitcrush {
        #[serde(default = "bits_default")]
        bits: f64,
        #[serde(default = "downsample_default")]
        downsample: f64,
    },
    /// Repeat short slices (stutter edit). `slice` seconds, repeats within [start, end] local time.
    Stutter { start: f64, end: f64, slice: f64 },
    /// Gain in decibels.
    Gain { db: Property<f64> },
}

fn q_default() -> f64 {
    0.707
}
fn delay_default() -> f64 {
    0.25
}
fn feedback_default() -> f64 {
    0.35
}
fn mix_default() -> f64 {
    0.35
}
fn drive_default() -> f64 {
    4.0
}
fn bits_default() -> f64 {
    8.0
}
fn downsample_default() -> f64 {
    4.0
}

// ------------------------------------------------------------------------------------------------
// Text & shapes
// ------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TextSource {
    pub text: String,
    /// Font family name (system font or imported font asset family). Fallbacks are automatic.
    #[serde(default = "default_font")]
    pub font: String,
    /// CSS-like weight (100..900).
    #[serde(default = "default_weight")]
    pub weight: u16,
    #[serde(default, skip_serializing_if = "is_default")]
    pub italic: bool,
    /// Font size in pixels.
    #[serde(default = "default_font_size")]
    pub size: f64,
    #[serde(default = "white_prop")]
    pub color: Property<Color>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub align: TextAlign,
    /// Line height multiplier.
    #[serde(default = "default_line_height")]
    pub line_height: f64,
    /// Extra spacing between characters (pixels). Animatable (tracking animation).
    #[serde(default = "zero_prop", skip_serializing_if = "is_zero_prop")]
    pub letter_spacing: Property<f64>,
    /// Wrap width in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<TextStroke>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<TextShadow>,
    /// Background box behind the text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<TextBackground>,
    /// Vertical gradient fill (overrides color): [top, bottom].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient: Option<[Color; 2]>,
    /// Per-unit (char/word/line) animation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animator: Option<TextAnimator>,
    /// Uppercase the text.
    #[serde(default, skip_serializing_if = "is_default")]
    pub uppercase: bool,
}

fn default_font() -> String {
    "sans-serif".into()
}
fn default_weight() -> u16 {
    700
}
fn default_font_size() -> f64 {
    96.0
}
fn default_line_height() -> f64 {
    1.15
}
fn white_prop() -> Property<Color> {
    Property::Static(Color::WHITE)
}

impl TextSource {
    pub fn simple(text: &str, size: f64) -> Self {
        TextSource {
            text: text.into(),
            font: default_font(),
            weight: default_weight(),
            italic: false,
            size,
            color: white_prop(),
            align: TextAlign::Center,
            line_height: default_line_height(),
            letter_spacing: zero_prop(),
            max_width: None,
            stroke: None,
            shadow: None,
            background: None,
            gradient: None,
            animator: None,
            uppercase: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TextStroke {
    pub color: Color,
    /// Stroke width in pixels (outside the glyph).
    pub width: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TextShadow {
    #[serde(default = "shadow_color")]
    pub color: Color,
    #[serde(default = "shadow_offset")]
    pub offset: Vec2,
    /// Blur radius in pixels.
    #[serde(default = "shadow_blur")]
    pub blur: f64,
}
fn shadow_color() -> Color {
    Color([0.0, 0.0, 0.0, 0.6])
}
fn shadow_offset() -> Vec2 {
    Vec2([4.0, 4.0])
}
fn shadow_blur() -> f64 {
    8.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TextBackground {
    pub color: Color,
    #[serde(default = "bg_padding")]
    pub padding: Vec2,
    #[serde(default)]
    pub radius: f64,
}
fn bg_padding() -> Vec2 {
    Vec2([24.0, 12.0])
}

/// Animates text units from a "from" state into place (and optionally out again).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TextAnimator {
    #[serde(default)]
    pub unit: TextUnit,
    /// Delay between consecutive units (seconds).
    #[serde(default = "stagger_default")]
    pub stagger: f64,
    /// Animation duration of each unit (seconds). 0 = instant (typewriter).
    #[serde(default = "unit_dur_default")]
    pub duration: f64,
    /// Local time at which the in-animation starts.
    #[serde(default)]
    pub start: f64,
    #[serde(default = "anim_ease_default")]
    pub ease: Easing,
    /// Order in which units animate.
    #[serde(default)]
    pub order: TextOrder,
    /// Starting state of each unit (animates to rest).
    #[serde(default)]
    pub from: UnitState,
    /// Optional exit animation, ending at the clip end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out: Option<TextOut>,
    /// Continuous sine wave offset per unit: [amplitude_px, frequency_hz, phase_per_unit].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wave: Option<[f64; 3]>,
    /// Random per-unit jitter in pixels, changing `jitter_rate` times per second (glitchy text).
    #[serde(default, skip_serializing_if = "is_default")]
    pub jitter: f64,
    #[serde(default = "jitter_rate_default")]
    pub jitter_rate: f64,
}

fn stagger_default() -> f64 {
    0.04
}
fn unit_dur_default() -> f64 {
    0.35
}
fn anim_ease_default() -> Easing {
    Easing::EaseOutBack
}
fn jitter_rate_default() -> f64 {
    12.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TextOut {
    #[serde(default = "stagger_default")]
    pub stagger: f64,
    #[serde(default = "unit_dur_default")]
    pub duration: f64,
    #[serde(default = "out_ease_default")]
    pub ease: Easing,
    #[serde(default)]
    pub to: UnitState,
}
fn out_ease_default() -> Easing {
    Easing::EaseInCubic
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TextUnit {
    #[default]
    Char,
    Word,
    Line,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TextOrder {
    #[default]
    Forward,
    Backward,
    CenterOut,
    EdgesIn,
    Random,
}

/// Per-unit animation state (deltas from rest).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct UnitState {
    #[serde(default)]
    pub opacity: f64,
    /// Offset in pixels.
    #[serde(default)]
    pub offset: Vec2,
    /// Scale factor.
    #[serde(default = "one_f")]
    pub scale: f64,
    /// Rotation in degrees.
    #[serde(default)]
    pub rotation: f64,
}
fn one_f() -> f64 {
    1.0
}

impl Default for UnitState {
    fn default() -> Self {
        UnitState { opacity: 0.0, offset: Vec2::ZERO, scale: 1.0, rotation: 0.0 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShapeSource {
    pub shape: ShapeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<Fill>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<ShapeStroke>,
    /// Trim paths: draw only [start, end] fraction of the outline (stroke draw-on animations).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trim: Option<Trim>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ShapeKind {
    Rect {
        size: Vec2,
        #[serde(default)]
        radius: f64,
    },
    Ellipse {
        size: Vec2,
    },
    /// Regular polygon (triangle = 3, hexagon = 6...).
    Polygon {
        sides: u32,
        radius: f64,
    },
    Star {
        points: u32,
        outer_radius: f64,
        inner_radius: f64,
    },
    Line {
        from: Vec2,
        to: Vec2,
    },
    /// Ring / donut.
    Ring {
        outer_radius: f64,
        inner_radius: f64,
    },
    /// SVG path data in local pixels (centered at 0,0).
    Path {
        d: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Fill {
    Solid(Color),
    Gradient(GradientFill),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GradientFill {
    #[serde(default)]
    pub kind: GradientKind,
    /// [offset 0..1, color] pairs.
    pub stops: Vec<(f64, Color)>,
    /// Angle in degrees for linear gradients.
    #[serde(default)]
    pub angle: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GradientKind {
    #[default]
    Linear,
    Radial,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShapeStroke {
    pub color: Color,
    pub width: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dash: Vec<f64>,
    #[serde(default)]
    pub cap: LineCap,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LineCap {
    #[default]
    Round,
    Butt,
    Square,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Trim {
    #[serde(default = "zero_prop")]
    pub start: Property<f64>,
    #[serde(default = "one_prop")]
    pub end: Property<f64>,
    /// Offset (0..1) rotating where the path starts.
    #[serde(default = "zero_prop")]
    pub offset: Property<f64>,
}

// ------------------------------------------------------------------------------------------------
// Blend modes
// ------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BlendMode {
    #[default]
    Normal,
    Add,
    Screen,
    Multiply,
    Overlay,
    SoftLight,
    HardLight,
    ColorDodge,
    ColorBurn,
    LinearDodge,
    LinearBurn,
    LinearLight,
    VividLight,
    PinLight,
    HardMix,
    Darken,
    Lighten,
    DarkerColor,
    LighterColor,
    Difference,
    Exclusion,
    Subtract,
    Divide,
    Hue,
    Saturation,
    Color,
    Luminosity,
    /// Only shows where the destination has alpha (stencil).
    StencilAlpha,
    /// Cuts out the destination where this layer has alpha.
    SilhouetteAlpha,
}

impl BlendMode {
    pub const ALL: &'static [BlendMode] = &[
        BlendMode::Normal,
        BlendMode::Add,
        BlendMode::Screen,
        BlendMode::Multiply,
        BlendMode::Overlay,
        BlendMode::SoftLight,
        BlendMode::HardLight,
        BlendMode::ColorDodge,
        BlendMode::ColorBurn,
        BlendMode::LinearDodge,
        BlendMode::LinearBurn,
        BlendMode::LinearLight,
        BlendMode::VividLight,
        BlendMode::PinLight,
        BlendMode::HardMix,
        BlendMode::Darken,
        BlendMode::Lighten,
        BlendMode::DarkerColor,
        BlendMode::LighterColor,
        BlendMode::Difference,
        BlendMode::Exclusion,
        BlendMode::Subtract,
        BlendMode::Divide,
        BlendMode::Hue,
        BlendMode::Saturation,
        BlendMode::Color,
        BlendMode::Luminosity,
        BlendMode::StencilAlpha,
        BlendMode::SilhouetteAlpha,
    ];

    /// Index used by the GPU blend shader.
    pub fn index(&self) -> u32 {
        Self::ALL.iter().position(|b| b == self).unwrap_or(0) as u32
    }

    pub fn name(&self) -> String {
        serde_json::to_value(self).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default()
    }
}

// ------------------------------------------------------------------------------------------------
// Assets
// ------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Asset {
    /// File path (absolute, or relative to the project file). For image sequences use a pattern
    /// like `frames/img_%04d.png` or a directory.
    pub path: String,
    #[serde(default)]
    pub kind: AssetKind,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// Probed metadata (filled on import).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub info: Option<MediaInfo>,
    /// Frame rate for image sequences.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence_fps: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    /// Detect from the file.
    #[default]
    Auto,
    Video,
    Audio,
    Image,
    /// Animated GIF / WebP / APNG.
    AnimatedImage,
    Svg,
    ImageSequence,
    Font,
    /// 3D LUT (.cube) for the `lut` effect.
    Lut,
    /// Lyrics / subtitles (.lrc, .srt, .vtt, .ass).
    Subtitles,
    /// Arbitrary data (JSON/CSV/text), readable from scripts.
    Data,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MediaInfo {
    #[serde(default, skip_serializing_if = "is_default")]
    pub width: u32,
    #[serde(default, skip_serializing_if = "is_default")]
    pub height: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fps: Option<f64>,
    /// Duration in seconds (None for still images).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub has_video: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub has_audio: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_codec: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_codec: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channels: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frames: Option<u64>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub has_alpha: bool,
    /// Font family names (for font assets).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub families: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_project_parses() {
        let json = r##"{
            "compositions": { "main": { "width": 1920, "height": 1080, "fps": 24, "duration": 10,
              "tracks": [ { "id": "t1", "clips": [
                 { "id": "c1", "start": 0, "duration": 2, "source": { "type": "solid", "color": "#ff0088" } },
                 { "id": "c2", "start": 2, "duration": 2, "source": { "type": "text", "text": "HELLO" },
                   "transform": { "scale": { "keyframes": [[0, 1.4], [0.3, 1, "punch"]] } },
                   "effects": [ { "effect": "glow", "params": { "intensity": 2 } } ] }
              ] } ] } }
        }"##;
        let p: Project = serde_json::from_str(json).unwrap();
        assert_eq!(p.root, "main");
        let c = &p.compositions["main"].tracks[0].clips[1];
        assert!(c.transform.scale.is_animated());
        let s = serde_json::to_string(&p).unwrap();
        let back: Project = serde_json::from_str(&s).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn speed_integration() {
        let mut c = Clip::new("c", 0.0, 4.0, ClipSource::Adjustment);
        c.speed = Property::Static(2.0);
        assert!((c.source_time(1.0, None) - 2.0).abs() < 1e-9);
        c.speed = serde_json::from_str(r#"{"keyframes":[[0,1],[2,1]]}"#).unwrap();
        assert!((c.source_time(1.5, None) - 1.5).abs() < 1e-6);
        c.speed = serde_json::from_str(r#"{"keyframes":[[0,0],[2,2]]}"#).unwrap();
        // integral of t from 0..2 = 2
        assert!((c.source_time(2.0, None) - 2.0).abs() < 1e-3);
    }
}
