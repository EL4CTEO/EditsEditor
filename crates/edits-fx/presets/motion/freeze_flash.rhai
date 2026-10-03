//! id = "freeze_flash"
//! name = "Freeze Frame Flash"
//! scope = "clip"
//! category = "motion"
//! description = "Freezes the picture at a hit with a flash and zoom punch for the rest of the clip (splits the clip)."
//! tags = ["freeze", "frame", "flash", "hit", "anime"]
//! params = [
//!   { name = "at", type = "float", default = 1.0, desc = "Clip-local freeze time" },
//!   { name = "zoom", type = "float", default = 0.15 },
//! ]
if args.at <= 0.01 || args.at >= ctx.clip_duration - 0.01 { log("freeze time must be inside the clip"); return; }
let speed = if type_of(clip.speed) == "map" { 1.0 } else { clip.speed ?? 1.0 };
let src_in = clip["in"] ?? 0.0;
let right = project.split(ctx.clip_id, ctx.clip_start + args.at);
project.set(right, "freeze", src_in + args.at * speed);
project.apply_preset(right, "flash_hit", #{ at: 0.0 });
project.apply_preset(right, "zoom_punch", #{ at: 0.0, strength: args.zoom, duration: 0.6 });
project.add_effect(right, fx("color_grade", #{ look: "bleach_bypass", amount: 0.6 }));
