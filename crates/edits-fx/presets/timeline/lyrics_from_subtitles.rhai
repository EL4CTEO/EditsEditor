//! id = "lyrics_from_subtitles"
//! name = "Lyrics from Subtitles"
//! scope = "timeline"
//! category = "timeline"
//! description = "Creates animated text clips from an .lrc/.srt/.vtt/.ass asset."
//! tags = ["lyrics", "subtitles", "karaoke", "text", "mv"]
//! params = [
//!   { name = "asset", type = "string", default = "", desc = "Subtitles asset id" },
//!   { name = "style", type = "enum", options = ["slide_up", "pop", "typewriter", "fade", "glitch", "beat_words", "plain"], default = "slide_up" },
//!   { name = "size", type = "float", default = 0.0, desc = "Font size px (0 = auto)" },
//!   { name = "y", type = "float", default = 0.36, desc = "Vertical position as fraction of height from center (+ = lower)" },
//!   { name = "font", type = "string", default = "sans-serif" },
//!   { name = "color", type = "color", default = "#ffffff" },
//!   { name = "stroke", type = "bool", default = true },
//! ]
if args.asset == "" { throw "pass the subtitles asset id"; }
let comp = project.comp();
let size = if args.size > 0.0 { args.size } else { comp.height * 0.065 };
let tr = project.add_track("lyrics");
let cues = subtitles(args.asset);
for c in cues {
    let src = #{ type: "text", text: c.text, size: size, font: args.font, color: args.color, max_width: comp.width * 0.85,
        shadow: #{ color: "#000000aa", offset: [3.0, 3.0], blur: 6.0 } };
    if args.stroke { src.stroke = #{ color: "#000000", width: size * 0.06 }; }
    let id = project.add_clip(tr, #{ start: c.start, duration: max(c.end - c.start, 0.3), source: src,
        transform: #{ position: [0.0, comp.height * args.y] } });
    let p = switch args.style { "slide_up" => "text_slide_up", "pop" => "text_pop", "typewriter" => "text_typewriter",
        "fade" => "text_fade_words", "glitch" => "text_glitch", "beat_words" => "text_beat_words", _ => "" };
    if p != "" { project.apply_preset(id, p, #{}); }
}
log(cues.len() + " lyric lines");
cues.len()
