//! id = "offset"
//! name = "Offset (Wrap Scroll)"
//! kind = "filter"
//! category = "distort"
//! description = "Scroll the image with wrap-around (infinite pans, seamless loops)."
//! tags = ["offset", "scroll", "wrap", "loop"]
//! params = [
//!   { name = "shift", type = "vec2", default = [0.0, 0.0], desc = "Shift in frame units (1 = full width/height)" },
//!   { name = "speed", type = "vec2", default = [0.0, 0.0], desc = "Additional shift per second" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    return src_wrap(uv - p.shift - p.speed * time());
}
