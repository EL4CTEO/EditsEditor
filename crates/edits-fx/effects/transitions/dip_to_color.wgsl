//! id = "dip_to_color"
//! name = "Dip to Color"
//! kind = "transition"
//! category = "basic"
//! description = "Fade out to a color (black/white) then into the next clip."
//! tags = ["dip", "fade to black", "fade to white", "basic"]
//! params = [
//!   { name = "color", type = "color", default = "#000000" },
//!   { name = "hold", type = "float", default = 0.1, min = 0.0, max = 0.9, desc = "Fraction of time fully on the color" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let h = p.hold * 0.5;
    let col = vec4f(p.color.rgb, 1.0);
    if (t < 0.5) {
        return mix(from_img(uv), col, smoothstep(0.0, 0.5 - h, t));
    }
    return mix(col, to_img(uv), smoothstep(0.5 + h, 1.0, t));
}
