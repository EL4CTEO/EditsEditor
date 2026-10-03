//! id = "crossfade"
//! name = "Crossfade"
//! kind = "transition"
//! category = "basic"
//! description = "Standard dissolve between clips."
//! tags = ["dissolve", "fade", "basic"]
//! params = []
fn effect(uv: vec2f, p: Params) -> vec4f {
    return mix(from_img(uv), to_img(uv), progress());
}
