//! id = "iris_in"
//! name = "Iris Reveal"
//! scope = "clip"
//! category = "entrance"
//! description = "Circular mask opens from the center to reveal the clip."
//! tags = ["iris", "circle", "mask", "reveal"]
//! params = [ { name = "duration", type = "float", default = 0.6 }, { name = "feather", type = "float", default = 30.0 } ]
let d = sqrt(ctx.width * ctx.width + ctx.height * ctx.height) * 1.1;
clip.masks = [#{ shape: #{ type: "ellipse", size: keys([[0.0, 0.0, "ease_in_out_cubic"], [args.duration, d]]) }, feather: args.feather }];
