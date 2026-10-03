//! id = "text_sticker"
//! name = "Sticker Text"
//! scope = "clip"
//! category = "text"
//! description = "Thick outline + drop shadow + pop-in, like a sticker or variety-show caption."
//! tags = ["text", "sticker", "outline", "caption", "variety"]
//! params = [ { name = "outline", type = "color", default = "#000000" }, { name = "width", type = "float", default = 8.0 } ]
if clip.source.type != "text" { log("needs a text clip"); return; }
clip.source.stroke = #{ color: args.outline, width: args.width };
clip.source.shadow = #{ color: "#00000099", offset: [6.0, 6.0], blur: 4.0 };
project.apply_preset(ctx.clip_id, "pop_in", #{});
