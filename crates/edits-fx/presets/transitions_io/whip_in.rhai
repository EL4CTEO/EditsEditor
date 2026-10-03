//! id = "whip_in"
//! name = "Whip In"
//! scope = "clip"
//! category = "entrance"
//! description = "Arrives with a whip-pan motion smear."
//! tags = ["whip", "pan", "entrance", "motion blur"]
//! params = [
//!   { name = "from", type = "enum", options = ["left", "right", "up", "down"], default = "right" },
//!   { name = "duration", type = "float", default = 0.3 },
//! ]
let horizontal = args.from == "left" || args.from == "right";
let dist = if horizontal { ctx.width } else { ctx.height };
clip.set_path("transform.position", keys([[0.0, dir_vec(args.from, dist), "whip"], [args.duration, [0.0, 0.0]]]));
clip.add_fx(fx("directional_blur", #{ length: keys([[0.0, 200.0, "ease_out_quad"], [args.duration, 0.0]]), angle: if horizontal { 0.0 } else { 90.0 } }));
